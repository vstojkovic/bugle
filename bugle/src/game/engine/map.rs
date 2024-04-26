use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::io::{Seek, SeekFrom};
use std::ops::{Deref, Index};
use std::path::{Path, PathBuf};

use anyhow::Result;
use binread::BinReaderExt;
use slog::{trace, Logger};

use super::name::{Name, NameRegistry};
use super::pak::Archive;
use super::uasset::{
    Export, ExportReader, ExportRef, ImportRef, Package, ResourceIndex, ResourceRef,
};
use super::UString;

#[derive(Debug)]
pub struct MapInfo {
    pub display_name: String,
    pub asset_path: String,
    pub object_name: String,
    pub db_name: PathBuf,
}

#[derive(Debug)]
pub struct MapEntry {
    pub id: usize,
    pub info: MapInfo,
}

impl Deref for MapEntry {
    type Target = MapInfo;
    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

#[derive(Debug)]
pub struct Maps {
    maps: Vec<MapEntry>,
    by_object_name: HashMap<String, usize>,
    by_asset_path: HashMap<String, usize>,
}

impl Maps {
    pub fn new() -> Self {
        Self {
            maps: Vec::new(),
            by_object_name: HashMap::new(),
            by_asset_path: HashMap::new(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &MapEntry> {
        self.maps.iter()
    }

    pub fn by_object_name<Q>(&self, object_name: &Q) -> Option<&MapEntry>
    where
        Q: Hash + Eq + ?Sized,
        String: Borrow<Q>,
    {
        self.by_object_name
            .get(object_name)
            .and_then(|&id| self.maps.get(id))
    }

    pub fn by_asset_path<Q>(&self, asset_path: &Q) -> Option<&MapEntry>
    where
        Q: Hash + Eq + ?Sized,
        String: Borrow<Q>,
    {
        self.by_asset_path
            .get(asset_path)
            .and_then(|&id| self.maps.get(id))
    }

    fn add(&mut self, map: MapInfo) -> Option<&MapEntry> {
        if self.by_object_name.contains_key(&map.object_name) {
            return None;
        }
        let id = self.maps.len();
        self.by_object_name.insert(map.object_name.clone(), id);
        self.by_asset_path.insert(map.asset_path.clone(), id);
        self.maps.push(MapEntry { id, info: map });
        Some(&self.maps[id])
    }
}

impl Index<usize> for Maps {
    type Output = MapEntry;
    fn index(&self, index: usize) -> &Self::Output {
        &self.maps[index]
    }
}

struct InterredNames {
    script_core_uobject: Name,
    map_data_table: Name,
    map_data_table_outer: Name,
    data_table: Name,
    class: Name,
    map_data_row: Name,
    script_struct: Name,
    row_struct: Name,
    map_name: Name,
    map_world: Name,
    db_name: Name,
}

pub struct MapExtractor {
    logger: Logger,
    name_registry: NameRegistry,
    names: InterredNames,
}

impl MapExtractor {
    pub fn new(logger: &Logger) -> Self {
        let mut name_registry = NameRegistry::new();

        let names = InterredNames {
            script_core_uobject: name_registry.inter("/Script/CoreUObject".into()),
            map_data_table: name_registry.inter("MapDataTable".into()),
            map_data_table_outer: name_registry.inter("/Game/Base/AlwaysCook/MapDataTable".into()),
            data_table: name_registry.inter("DataTable".into()),
            class: name_registry.inter("Class".into()),
            map_data_row: name_registry.inter("MapDataRow".into()),
            script_struct: name_registry.inter("ScriptStruct".into()),
            row_struct: name_registry.inter("RowStruct".into()),
            map_name: name_registry.inter("MapName".into()),
            map_world: name_registry.inter("MapWorld".into()),
            db_name: name_registry.inter("DBName".into()),
        };

        Self {
            logger: logger.clone(),
            name_registry,
            names,
        }
    }

    pub fn extract_mod_maps<P: AsRef<Path>>(&self, pak_path: P, maps: &mut Maps) -> Result<()> {
        trace!(self.logger, "Extracting maps from mod"; "pak_path" => pak_path.as_ref().to_str());

        let pak = Archive::new(pak_path)?;
        let preload_pkgs = gather_preload_packages(&self.logger, &pak);

        let mut map_data_candidates = Vec::new();
        for path in preload_pkgs {
            let pkg = Package::new(&pak, &path, &self.name_registry)?;
            self.gather_map_data_candidates(&pkg, &mut map_data_candidates);
        }

        for pkg_name in map_data_candidates {
            let pkg = Package::new(&pak, &pkg_name, &self.name_registry)?;
            self.gather_pkg_maps(&pkg, maps)?;
        }

        Ok(())
    }

    pub fn extract_base_game_maps<P: AsRef<Path>>(
        &self,
        pak_path: P,
        maps: &mut Maps,
    ) -> Result<()> {
        let pak = Archive::new(pak_path)?;

        let pkg = Package::new(&pak, BASE_MAP_DATA_TABLE, &self.name_registry)?;
        self.gather_pkg_maps(&pkg, maps)?;

        Ok(())
    }

    fn gather_map_data_candidates(&self, preload_pkg: &Package, map_data_pkgs: &mut Vec<String>) {
        trace!(self.logger, "Gathering map data candidates"; "preload_pkg" => preload_pkg.path());

        if !preload_pkg
            .iter_imports()
            .any(|imp| self.is_map_data_table_import(imp))
        {
            return;
        }

        for imp in preload_pkg.iter_imports() {
            if imp.name().text().starts_with("/Game/Mods/")
                && *imp.package() == self.names.script_core_uobject
            {
                map_data_pkgs.push(imp.name().text().strip_prefix("/Game/").unwrap().to_owned());
            }
        }
    }

    fn is_map_data_table_import(&self, imp: ImportRef) -> bool {
        if let ResourceRef::Import(outer) = imp.outer() {
            *imp.name() == self.names.map_data_table
                && *outer.name() == self.names.map_data_table_outer
        } else {
            false
        }
    }

    fn gather_pkg_maps(&self, pkg: &Package, maps: &mut Maps) -> Result<()> {
        trace!(self.logger, "Gathering package maps"; "pkg" => pkg.path());

        let mut data_table_imp = None;
        let mut map_data_row_imp = None;

        for imp in pkg.iter_imports() {
            if *imp.name() == self.names.data_table && *imp.class() == self.names.class {
                data_table_imp = Some(imp);
            } else if *imp.name() == self.names.map_data_row
                && *imp.class() == self.names.script_struct
            {
                map_data_row_imp = Some(imp);
            }

            if data_table_imp.is_some() && map_data_row_imp.is_some() {
                break;
            }
        }

        let data_table_imp: ResourceIndex = if let Some(imp) = data_table_imp {
            imp.into()
        } else {
            return Ok(());
        };
        let map_data_row_imp: ResourceIndex = if let Some(imp) = map_data_row_imp {
            imp.into()
        } else {
            return Ok(());
        };

        for exp in pkg.iter_exports() {
            if exp.class != data_table_imp {
                continue;
            }

            self.gather_export_maps(pkg, exp, map_data_row_imp, maps)?;
        }

        Ok(())
    }

    fn gather_export_maps(
        &self,
        pkg: &Package,
        exp: ExportRef,
        map_data_row_imp: ResourceIndex,
        maps: &mut Maps,
    ) -> Result<()> {
        {
            let exp: &Export = &exp;
            trace!(self.logger, "Gathering maps from export"; "export" => ?exp);
        }

        let mut exp = pkg.open_export(exp.index())?;
        let mut found_row_struct = false;
        while let Some(prop) = exp.read_property_tag()? {
            if !found_row_struct && *prop.name == self.names.row_struct {
                found_row_struct = true;
                let row_struct: ResourceIndex = exp.read_le()?;
                if row_struct != map_data_row_imp {
                    return Ok(());
                }
            } else {
                exp.skip_property(&prop)?;
            }
        }
        if !found_row_struct {
            return Ok(());
        }

        if exp.read_le::<u32>()? != 0 {
            // skip object GUID
            exp.seek(SeekFrom::Current(16))?;
        }

        let num_rows: u32 = exp.read_le()?;
        trace!(self.logger, "Extracting map info from table"; "num_rows" => num_rows);
        for i in (0..num_rows).rev() {
            self.extract_row_map_info(&mut exp, pkg, i == 0, maps)?;
        }

        Ok(())
    }

    fn extract_row_map_info(
        &self,
        exp: &mut ExportReader,
        pkg: &Package,
        last_row: bool,
        maps: &mut Maps,
    ) -> Result<()> {
        trace!(self.logger, "Extracting map info from table row");

        // skip the row name
        exp.seek(SeekFrom::Current(8))?;

        let mut display_name = None;
        let mut asset_path = None;
        let mut object_name = None;
        let mut db_name = None;

        while let Some(prop) = exp.read_property_tag()? {
            if *prop.name == self.names.map_name {
                display_name = Some(exp.read_le::<UString>()?.into());
            } else if *prop.name == self.names.map_world {
                let map_world: String = exp.read_le::<UString>()?.into();
                if let Some((asset_part, object_part)) = map_world.split_once('.') {
                    asset_path = Some(asset_part.to_owned());
                    object_name = Some(object_part.to_owned());
                }
            } else if *prop.name == self.names.db_name {
                db_name = Some(pkg.name_ref(exp.read_le()?).to_string());
            } else {
                exp.skip_property(&prop)?;
            }

            if last_row
                && display_name.is_some()
                && asset_path.is_some()
                && object_name.is_some()
                && db_name.is_some()
            {
                break;
            }
        }

        let display_name = if let Some(name) = display_name {
            name
        } else {
            return Ok(());
        };
        let asset_path = if let Some(path) = asset_path {
            path
        } else {
            return Ok(());
        };
        let object_name = if let Some(name) = object_name {
            name
        } else {
            return Ok(());
        };
        let db_name = if let Some(name) = db_name {
            let mut db_name = PathBuf::with_capacity(name.len() + 3);
            db_name.set_file_name(name);
            db_name.set_extension("db");
            db_name
        } else {
            return Ok(());
        };

        maps.add(MapInfo {
            display_name,
            asset_path,
            object_name,
            db_name,
        });

        Ok(())
    }
}

fn gather_preload_packages(logger: &Logger, pak: &Archive) -> Vec<String> {
    trace!(logger, "Gathering preload packages"; "pak_path" => pak.path().to_str());
    pak.iter()
        .filter(|entry| !entry.encrypted && entry.path.contains("/PreLoad/"))
        .filter_map(|entry| entry.path.strip_suffix(".uasset").map(str::to_owned))
        .collect()
}

const BASE_MAP_DATA_TABLE: &'static str = "ConanSandbox/Content/Base/AlwaysCook/MapDataTable";
