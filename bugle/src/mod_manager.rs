use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use bit_vec::BitVec;
use dynabus::Bus;
use lazy_static::lazy_static;
use regex::Regex;
use slog::{warn, Logger};

use crate::bus::AppBus;
use crate::config::{ConfigManager, ModMismatchChecks};
use crate::game::platform::steam::PlatformReady;
use crate::game::platform::ModDirectory;
use crate::game::{list_mod_controllers, Game, ModEntry, ModRef, Mods};
use crate::gui::{prompt_confirm, ModUpdateProgressDialog, ModUpdateSelectionDialog};
use crate::util::weak_cb;

pub struct ModManager {
    logger: Logger,
    config: Rc<ConfigManager>,
    game: Arc<Game>,
    mod_directory: Rc<dyn ModDirectory>,
}

impl ModManager {
    pub fn new(
        logger: &Logger,
        config: Rc<ConfigManager>,
        bus: Rc<RefCell<AppBus>>,
        game: Arc<Game>,
        mod_directory: Rc<dyn ModDirectory>,
    ) -> Rc<Self> {
        let logger = logger.clone();

        let this = Rc::new(Self {
            logger,
            config,
            game,
            mod_directory,
        });

        {
            let mut bus = bus.borrow_mut();
            bus.subscribe_observer(weak_cb!([this] => |&PlatformReady| this.check_mod_updates()));
        }

        this
    }

    pub fn check_mod_updates(&self) {
        if !Rc::clone(&self.mod_directory).can_update() {
            return;
        }

        for entry in self.game.installed_mods().iter() {
            match Rc::clone(&self.mod_directory).needs_update(entry) {
                Ok(needs_update) => entry.set_needs_update(needs_update),
                Err(err) => warn!(
                    self.logger,
                    "Error checking whether mod needs update";
                    "mod_name" => entry.info.as_ref().map(|info| info.name.as_str()).unwrap_or("???"),
                    "pak_path" => ?entry.pak_path,
                    "error" => %err,
                ),
            }
        }
    }

    pub fn import_mod_list(&self, path: &Path) -> Result<Vec<ModRef>> {
        let active_mods = self.game.load_mod_list_from(&path)?;
        self.game.save_mod_list(&active_mods)?;
        Ok(active_mods)
    }

    pub fn outdated_active_mods(&self) -> Result<Vec<ModRef>> {
        let mod_list = self.game.load_mod_list()?;
        self.check_mod_updates();

        let installed_mods = self.game.installed_mods();
        let mut outdated_mods = Vec::new();
        for mod_ref in mod_list {
            if let Some(entry) = installed_mods.get(&mod_ref) {
                if entry.needs_update() {
                    outdated_mods.push(mod_ref);
                }
            }
        }

        Ok(outdated_mods)
    }

    pub fn update_mods(&self, outdated_mods: Vec<ModRef>) {
        if outdated_mods.is_empty() || !Rc::clone(&self.mod_directory).can_update() {
            return;
        }

        let installed_mods = self.game.installed_mods();

        let dialog = ModUpdateSelectionDialog::new(
            fltk::app::first_window().as_ref().unwrap(),
            installed_mods,
            outdated_mods,
        );
        let mods_to_update = match dialog.run() {
            None => return,
            Some(mods) => mods,
        };
        if mods_to_update.is_empty() {
            return;
        }

        let dialog = ModUpdateProgressDialog::new(
            fltk::app::first_window().as_ref().unwrap(),
            installed_mods,
            mods_to_update,
            Rc::clone(&self.mod_directory),
        );
        dialog.run();
    }

    pub fn validate_single_player_mods(&self, map_id: usize) -> Result<bool> {
        if let ModMismatchChecks::Disabled = self.config.get().mod_mismatch_checks {
            return Ok(true);
        }

        let mod_list = self.game.load_mod_list()?;

        if let Some(mismatch) = self.detect_single_player_mod_mismatch(mod_list, map_id)? {
            let installed_mods = self.game.installed_mods();
            let prompt = format!(
                "{}{}{}",
                PROMPT_SP_MOD_MISMATCH,
                join_mod_names(TXT_MISSING_MODS, installed_mods, mismatch.missing_mods),
                join_mod_names(TXT_ADDED_MODS, installed_mods, mismatch.added_mods),
            );
            Ok(prompt_confirm(&prompt))
        } else {
            Ok(true)
        }
    }

    pub fn fix_mod_list(&self, mod_list: &mut Vec<ModRef>) -> bool {
        let installed_mods = self.game.installed_mods();
        let mut available_set = BitVec::from_elem(installed_mods.len(), true);

        for mod_ref in mod_list.iter() {
            if let ModRef::Installed(idx) = mod_ref {
                available_set.set(*idx, false);
            }
        }

        let mut fixed_all = true;
        for mod_ref in mod_list.iter_mut() {
            let pak_path = match mod_ref {
                ModRef::UnknownPakPath(path) => path,
                _ => continue,
            };
            let fixed_idx = installed_mods.iter().enumerate().find_map(|(idx, entry)| {
                if !available_set[idx] {
                    return None;
                }
                let root = installed_mods.root_for(entry.provenance)?;
                let suffix = entry.pak_path.strip_prefix(root).ok()?;
                if pak_path.ends_with(suffix) {
                    Some(idx)
                } else {
                    None
                }
            });
            if let Some(idx) = fixed_idx {
                *mod_ref = ModRef::Installed(idx);
                available_set.set(idx, false);
            } else {
                fixed_all = false;
            }
        }

        fixed_all
    }

    pub fn resolve_mods(&self, mods: &mut [(u64, Option<String>)]) {
        Rc::clone(&self.mod_directory).resolve(mods);
    }

    fn detect_single_player_mod_mismatch(
        &self,
        mod_list: Vec<ModRef>,
        map_id: usize,
    ) -> Result<Option<ModMismatch>> {
        let installed_mods = self.game.installed_mods();
        let mut active_mods: HashSet<ModRef> = mod_list.into_iter().collect();

        let db_path = self.game.in_progress_game_path(map_id);
        let db_metadata = std::fs::metadata(&db_path)?;
        let mod_controllers =
            if db_metadata.len() != 0 { list_mod_controllers(db_path)? } else { Vec::new() };

        let mut required_folders = HashMap::new();
        for controller in mod_controllers {
            if let Some(captures) = MOD_CTRL_FOLDER_REGEX.captures(&controller) {
                let folder = captures.get(1).unwrap().as_str();
                required_folders.insert(folder.to_string(), false);
            }
        }

        let mut added_mods = HashSet::new();
        for mod_ref in active_mods.drain() {
            let info = installed_mods
                .get(&mod_ref)
                .and_then(|entry| entry.info.as_ref().ok());
            if let Some(info) = info {
                if let Some(active) = required_folders.get_mut(&info.folder_name) {
                    *active = true;
                    continue;
                }
            }
            added_mods.insert(mod_ref);
        }

        let mut missing_mods = HashSet::new();
        for (folder, active) in required_folders.drain() {
            if !active {
                missing_mods.insert(installed_mods.by_folder(folder));
            }
        }

        if added_mods.is_empty() && missing_mods.is_empty() {
            Ok(None)
        } else {
            Ok(Some(ModMismatch {
                missing_mods,
                added_mods,
            }))
        }
    }
}

struct ModMismatch {
    missing_mods: HashSet<ModRef>,
    added_mods: HashSet<ModRef>,
}

fn push_name(s: &mut String, entry: &ModEntry) {
    if let Ok(info) = entry.info.as_ref() {
        s.push_str(&info.name);
    } else {
        s.push_str(&format!("??? ({})", entry.pak_path.display()));
    }
}

fn join_mod_names(heading: &str, mods: &Mods, refs: HashSet<ModRef>) -> String {
    let mut result = String::new();
    if refs.is_empty() {
        return result;
    }

    result.push_str("\n\n");
    result.push_str(heading);
    for mod_ref in refs {
        result.push('\n');
        match mod_ref {
            ModRef::Installed(idx) => push_name(&mut result, &mods[idx]),
            ModRef::Custom(entry) => push_name(&mut result, &entry),
            ModRef::UnknownFolder(folder) => result.push_str(&format!("??? ({})", folder)),
            ModRef::UnknownPakPath(path) => result.push_str(&format!("??? ({})", path.display())),
        };
    }
    result
}

const PROMPT_SP_MOD_MISMATCH: &str =
    "It looks like your mod list doesn't match this game. Launch anyway?";
const TXT_MISSING_MODS: &str = "Missing mods:";
const TXT_ADDED_MODS: &str = "Added mods:";

lazy_static! {
    static ref MOD_CTRL_FOLDER_REGEX: Regex = Regex::new("/Game/Mods/([^/]+)/.*").unwrap();
}
