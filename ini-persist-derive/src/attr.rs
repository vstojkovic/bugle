use quote::ToTokens;
use syn::meta::ParseNestedMeta;
use syn::{Attribute, Expr, ExprLit, ExprPath, Lit, Path, Result};

pub trait IniAttr: Sized + Default {
    fn update_from_ast(&mut self, attr: &Attribute) -> Result<()>;
    fn from_ast<'a, I: Iterator<Item = &'a Attribute>>(attrs: I) -> Result<Self> {
        let mut result = Self::default();
        for attr in attrs {
            if attr.path().is_ident("ini") {
                result.update_from_ast(attr)?;
            }
        }
        Ok(result)
    }
}

#[derive(Default)]
pub struct StructAttr {
    pub section: Option<Option<String>>,
}

#[derive(Default)]
pub struct FieldAttr {
    pub key: Option<String>,
    pub flatten: Option<()>,
    pub load_fn: Option<LoadFn>,
}

#[derive(Default)]
pub struct EnumAttr {
    pub repr: Option<()>,
}

pub enum LoadFn {
    InPlace(Path),
    Constructed(Path),
    Parsed(Path),
}

impl IniAttr for StructAttr {
    fn update_from_ast(&mut self, attr: &Attribute) -> Result<()> {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("section") {
                let section = extract_str(&meta)?;
                if self.section.is_some() {
                    return Err(meta.error("conflicting section specified"));
                }
                self.section = Some(Some(section));
                return Ok(());
            }
            if meta.path.is_ident("general") {
                if self.section.is_some() {
                    return Err(meta.error("conflicting section specified"));
                }
                self.section = Some(None);
                return Ok(());
            }
            Err(meta.error(format_args!(
                "unknown ini attribute `{}`",
                meta.path.to_token_stream()
            )))
        })?;
        Ok(())
    }
}

impl IniAttr for FieldAttr {
    fn update_from_ast(&mut self, attr: &Attribute) -> Result<()> {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("key") {
                let key = extract_str(&meta)?;
                if self.key.is_some() {
                    return Err(meta.error("duplicate ini_load attribute `key`"));
                }
                if self.flatten.is_some() {
                    return Err(meta.error("cannot define a key for a flattened field"));
                }
                self.key = Some(key);
                return Ok(());
            }
            if meta.path.is_ident("load_in_with") {
                let path = extract_path(&meta)?;
                if self.load_fn.is_some() {
                    return Err(meta.error("conflicting load function specified"));
                }
                if self.flatten.is_some() {
                    return Err(meta.error("cannot define a load function for a flattened field"));
                }
                self.load_fn = Some(LoadFn::InPlace(path));
                return Ok(());
            }
            if meta.path.is_ident("load_with") {
                let path = extract_path(&meta)?;
                if self.load_fn.is_some() {
                    return Err(meta.error("conflicting load function specified"));
                }
                if self.flatten.is_some() {
                    return Err(meta.error("cannot define a load function for a flattened field"));
                }
                self.load_fn = Some(LoadFn::Constructed(path));
                return Ok(());
            }
            if meta.path.is_ident("parse_with") {
                let path = extract_path(&meta)?;
                if self.load_fn.is_some() {
                    return Err(meta.error("conflicting load function specified"));
                }
                if self.flatten.is_some() {
                    return Err(meta.error("cannot define a load function for a flattened field"));
                }
                self.load_fn = Some(LoadFn::Parsed(path));
                return Ok(());
            }
            if meta.path.is_ident("flatten") {
                if self.key.is_some() {
                    return Err(meta.error("cannot flatten a field with a defined key"));
                }
                if self.load_fn.is_some() {
                    return Err(meta.error("cannot flatten a field with a defined load function"));
                }
                self.flatten = Some(());
                return Ok(());
            }
            Err(meta.error(format_args!(
                "unknown ini attribute `{}`",
                meta.path.to_token_stream()
            )))
        })?;
        Ok(())
    }
}

impl IniAttr for EnumAttr {
    fn update_from_ast(&mut self, attr: &Attribute) -> Result<()> {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("repr") {
                if self.repr.is_some() {
                    return Err(meta.error("duplicate ini property `repr`"));
                }
                self.repr = Some(());
                return Ok(());
            }
            Err(meta.error(format_args!(
                "unknown ini attribute `{}`",
                meta.path.to_token_stream()
            )))
        })?;
        Ok(())
    }
}

fn extract_str(meta: &ParseNestedMeta) -> Result<String> {
    extract_value(meta, |expr| match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(lit), ..
        }) => Ok(lit.value()),
        _ => Err(meta.error("expected a string literal")),
    })
}

fn extract_path(meta: &ParseNestedMeta) -> Result<Path> {
    extract_value(meta, |expr| match expr {
        Expr::Path(ExprPath { path, .. }) => Ok(path.clone()),
        Expr::Lit(ExprLit {
            lit: Lit::Str(lit), ..
        }) => Ok(lit.parse()?),
        _ => Err(meta.error("expected a path")),
    })
}

fn extract_value<R, F: FnOnce(&Expr) -> Result<R>>(
    meta: &ParseNestedMeta,
    extractor: F,
) -> Result<R> {
    let expr: Expr = meta.value()?.parse()?;
    let mut result = &expr;
    while let Expr::Group(group) = result {
        result = &group.expr;
    }
    extractor(result)
}
