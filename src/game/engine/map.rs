use std::io::{Seek, SeekFrom};
use std::path::Path;

use anyhow::Result;
use binread::BinReaderExt;

use super::name::{Name, NameRegistry};
use super::pak::Archive;
use super::uasset::{ExportReader, ImportRef, Package, ResourceIndex, ResourceRef};
use super::UString;

#[derive(Debug)]
#[allow(dead_code)]
pub struct MapInfo {
    pub display_name: String,
    pub asset_path: String,
    pub object_name: String,
    pub db_name: String,
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
    name_registry: NameRegistry,
    names: InterredNames,
}

impl MapExtractor {
    pub fn new() -> Self {
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
            name_registry,
            names,
        }
    }

    pub fn extract_mod_maps<P: AsRef<Path>>(
        &self,
        pak_path: P,
        maps: &mut Vec<MapInfo>,
    ) -> Result<()> {
        let pak = Archive::new(pak_path)?;
        let preload_pkgs = gather_preload_packages(&pak);

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
        maps: &mut Vec<MapInfo>,
    ) -> Result<()> {
        let pak = Archive::new(pak_path)?;

        let pkg = Package::new(&pak, BASE_MAP_DATA_TABLE, &self.name_registry)?;
        self.gather_pkg_maps(&pkg, maps)?;

        Ok(())
    }

    fn gather_map_data_candidates(&self, preload_pkg: &Package, map_data_pkgs: &mut Vec<String>) {
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

    fn gather_pkg_maps(&self, pkg: &Package, maps: &mut Vec<MapInfo>) -> Result<()> {
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

            let reader = pkg.open_export(exp.index())?;
            self.gather_export_maps(pkg, reader, map_data_row_imp, maps)?;
        }

        Ok(())
    }

    fn gather_export_maps(
        &self,
        pkg: &Package,
        mut exp: ExportReader,
        map_data_row_imp: ResourceIndex,
        maps: &mut Vec<MapInfo>,
    ) -> Result<()> {
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
        maps: &mut Vec<MapInfo>,
    ) -> Result<()> {
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
            name
        } else {
            return Ok(());
        };

        maps.push(MapInfo {
            display_name,
            asset_path,
            object_name,
            db_name,
        });

        Ok(())
    }
}

fn gather_preload_packages(pak: &Archive) -> Vec<String> {
    pak.iter()
        .filter(|entry| !entry.encrypted && entry.path.contains("/PreLoad/"))
        .filter_map(|entry| entry.path.strip_suffix(".uasset").map(str::to_owned))
        .collect()
}

const BASE_MAP_DATA_TABLE: &'static str = "ConanSandbox/Content/Base/AlwaysCook/MapDataTable";
