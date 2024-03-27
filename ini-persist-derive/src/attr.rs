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
pub struct NoAttrSupport;

impl IniAttr for NoAttrSupport {
    fn update_from_ast(&mut self, attr: &Attribute) -> Result<()> {
        attr.parse_nested_meta(unknown_attr)
    }
}

pub fn unknown_attr(meta: ParseNestedMeta) -> Result<()> {
    use quote::ToTokens;
    Err(meta.error(format_args!(
        "unknown ini attribute `{}`",
        meta.path.to_token_stream()
    )))
}

pub fn extract_str(meta: &ParseNestedMeta) -> Result<String> {
    extract_value(meta, |expr| match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(lit), ..
        }) => Ok(lit.value()),
        _ => Err(meta.error("expected a string literal")),
    })
}

pub fn extract_path(meta: &ParseNestedMeta) -> Result<Path> {
    extract_value(meta, |expr| match expr {
        Expr::Path(ExprPath { path, .. }) => Ok(path.clone()),
        Expr::Lit(ExprLit {
            lit: Lit::Str(lit), ..
        }) => Ok(lit.parse()?),
        _ => Err(meta.error("expected a path")),
    })
}

pub fn extract_value<R, F: FnOnce(&Expr) -> Result<R>>(
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
