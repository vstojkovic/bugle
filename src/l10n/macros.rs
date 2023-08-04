macro_rules! msg {
    (@attr) => { None };
    (@attr $attr:ident) => { Some(stringify!($attr).into()) };
    (@arg $args:ident $arg:ident) => {
        $args.set(stringify!($arg), $arg);
    };
    (@arg $args:ident $arg_key:ident $arg_value:expr) => {
        $args.set(stringify!($arg_key), $arg_value);
    };
    (@args) => { None };
    (@args $($arg_key:ident $($arg_value:expr)?)+) => {
        Some({
            let mut args = fluent_bundle::FluentArgs::new();
            $($crate::l10n::msg!(@arg args $arg_key $($arg_value)?);)+
            args
        })
    };
    ($key:ident $(.$attr:ident)? $($(, $arg_key:ident $(=> $arg_value:expr)?)+)?) => {
        $crate::l10n::LocalizableMessage {
            key: stringify!($key).into(),
            attr: $crate::l10n::msg!(@attr $($attr)?),
            args: $crate::l10n::msg!(@args $($($arg_key $($arg_value)?)+)?)
        }
    };
}
pub(crate) use msg;

macro_rules! err {
    ($key:ident $(.$attr:ident)? $($(, $arg_key:ident $(=> $arg_value:expr)?)+)?) => {
        $crate::l10n::ErrorMessage(std::sync::Mutex::new(
            $crate::l10n::msg!($key $(.$attr)? $($(, $arg_key $(=> $arg_value)?)+)?)
        ))
    };
}
pub(crate) use err;

macro_rules! use_l10n {
    (@ $macro_name:ident $localizer:expr => $d:tt) => {
        macro_rules! $macro_name {
            (&$key:ident) => {
                $localizer.value(stringify!($key)).as_ref()
            };
            ($key:ident) => {
                $localizer.value(stringify!($key)).into_owned()
            };
            ($key:ident $d(, $d arg_key:ident $d(=> $d arg_value:expr)?)+) => {
                {
                    let mut args = fluent_bundle::FluentArgs::new();
                    $d($crate::l10n::msg!(@arg args $d arg_key $d($d arg_value)?);)+
                    $localizer.format_value(stringify!($key), &args)
                }
            };
            (&$key:ident.$attr:ident) => {
                $localizer
                    .attr(stringify!($key), stringify!($attr))
                    .as_ref()
            };
            ($key:ident.$attr:ident) => {
                $localizer
                    .attr(stringify!($key), stringify!($attr))
                    .into_owned()
            };
            ($key:ident.$attr:ident $d(, $d arg_key:ident $d(=> $d arg_value:expr)?)+) => {
                {
                    let mut args = fluent_bundle::FluentArgs::new();
                    $d($crate::l10n::msg!(@arg args $d arg_key $d($d arg_value)?);)+
                    $localizer.format_attr(stringify!($key), stringify!($attr), &args)
                }
            };
        }
    };
    ($localizer:expr) => {
        use_l10n!($localizer => l10n);
    };
    ($localizer:expr => $macro_name:ident) => {
        use_l10n!(@ $macro_name $localizer => $);
    };
}
pub(crate) use use_l10n;
