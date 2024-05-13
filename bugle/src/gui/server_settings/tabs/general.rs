use std::cell::Cell;
use std::rc::Rc;

use chrono::TimeDelta;
use fltk::button::CheckButton;
use fltk::input::{Input, SecretInput};
use fltk::prelude::*;
use fltk_float::grid::Grid;
use fltk_float::scroll::Scrollable;
use fltk_float::{EmptyElement, LayoutElement};
use num::ToPrimitive;

use crate::game::settings::server::{
    CombatModeModifier, Community, EventLogPrivacy, GeneralSettings, OnlinePlayerInfoVisibility,
    PublicGeneralSettings,
};
use crate::game::settings::Nudity;
use crate::gui::server_settings::{
    make_label, DailyHoursInput, EditorBuilder, PrivateBuilder, SliderInput,
};
use crate::gui::widgets::DropDownList;
use crate::gui::{min_input_width, wrapper_factory};
use crate::servers::Mode;

use super::SettingsTab;

pub struct GeneralTab {
    root: Scrollable,
    battleye_required: Cell<bool>,
    mode_modifier: Cell<CombatModeModifier>,
    community: Community,
    max_ping: Option<u32>,
    pvp_enabled_prop: CheckButton,
    raid_enabled_prop: CheckButton,
    raid_restricted_prop: CheckButton,
    raid_hours_prop: DailyHoursInput,
    max_clan_size_prop: SliderInput,
    private_props: Option<PrivateProperties>,
}

struct PrivateProperties {
    motd_prop: Input,
    server_password_prop: SecretInput,
    admin_password_prop: SecretInput,
    pvp_restricted_prop: CheckButton,
    pvp_hours_prop: DailyHoursInput,
    dbd_enabled_prop: CheckButton,
    dbd_period_prop: SliderInput,
    no_ownership_prop: CheckButton,
    containers_ignore_ownership_prop: CheckButton,
    sandstorm_enabled_prop: CheckButton,
    clan_markers_enabled_prop: CheckButton,
    tether_distance_prop: SliderInput,
    max_nudity_prop: DropDownList,
    voice_chat_enabled_prop: CheckButton,
    enforce_whitelist_prop: CheckButton,
    claim_popup_disabled_prop: CheckButton,
    log_privacy_prop: DropDownList,
    family_share_allowed_prop: CheckButton,
    healthbar_distance_prop: SliderInput,
    online_info_visibility_prop: DropDownList,
}

impl GeneralTab {
    pub fn new(settings: &PublicGeneralSettings, include_private: bool) -> Rc<Self> {
        let input_width = min_input_width(&["23:59", "99999"]);

        let root = Scrollable::builder().with_gap(10, 10);

        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_row_spacing(5)
            .with_col_spacing(10);

        grid.col().add(); // label
        grid.col().with_stretch(1).add(); // checkbox
        grid.col().add(); // start label
        grid.col().with_min_size(input_width).add(); // start input
        grid.col().add(); // end label
        grid.col().with_min_size(input_width).add(); // end input

        let mut grid = PrivateBuilder::new(grid, include_private);

        let motd_prop = include_private.then(|| {
            grid.public.row().add();
            grid.public
                .cell()
                .unwrap()
                .wrap(make_label("Message of the day:"));
            grid.public.span(1, 5).unwrap().wrap(Input::default())
        });

        let server_password_prop = include_private.then(|| {
            grid.public.row().add();
            grid.public
                .cell()
                .unwrap()
                .wrap(make_label("Server password:"));
            grid.public.span(1, 5).unwrap().wrap(SecretInput::default())
        });

        let admin_password_prop = include_private.then(|| {
            grid.public.row().add();
            grid.public
                .cell()
                .unwrap()
                .wrap(make_label("Admin password:"));
            grid.public.span(1, 5).unwrap().wrap(SecretInput::default())
        });

        if include_private {
            grid.public.row().add();
            grid.public
                .span(1, 6)
                .unwrap()
                .with_top_padding(25)
                .add(EmptyElement);
        }

        let pvp_enabled_prop = grid.public.bool_prop("PVP enabled");
        let pvp_restricted_prop = grid.bool_prop("Time restrict PVP");
        let pvp_hours_prop = grid.daily_hours_prop("PVP allowed");

        if include_private {
            grid.public.row().add();
            grid.public
                .span(1, 6)
                .unwrap()
                .with_top_padding(25)
                .add(EmptyElement);
        }

        let raid_enabled_prop = grid.public.bool_prop("PVP building damage enabled");
        let raid_restricted_prop = grid.public.bool_prop("Time restrict building damage");
        let raid_hours_prop = grid.public.daily_hours_prop("Damage allowed");
        let dbd_enabled_prop = grid.bool_prop("Dynamic building damage");
        let dbd_period_prop = grid.range_prop("DBD period:", 1.0, 3600.0, 1.0, 1);

        if include_private {
            grid.public.row().add();
            grid.public
                .span(1, 6)
                .unwrap()
                .with_top_padding(25)
                .add(EmptyElement);
        }

        let no_ownership_prop = grid.bool_prop("No ownership");
        let containers_ignore_ownership_prop = grid.bool_prop("Containers ignore ownership");
        let sandstorm_enabled_prop = grid.bool_prop("Enable sandstorm");
        let clan_markers_enabled_prop = grid.bool_prop("Enable clan map markers");
        let max_clan_size_prop = grid.public.range_prop("Clan max size:", 1.0, 60.0, 1.0, 1);
        let tether_distance_prop = grid.range_prop("Tethering distance:", 12000.0, 52000.0, 1.0, 1);

        if include_private {
            grid.public.row().add();
            grid.public
                .span(1, 6)
                .unwrap()
                .with_top_padding(25)
                .add(EmptyElement);
        }

        let max_nudity_prop = grid.enum_prop("Maximum nudity:", &["None", "Partial", "Full"]);
        let voice_chat_enabled_prop = grid.bool_prop("Enable voice chat");
        let enforce_whitelist_prop = grid.bool_prop("Only allow whitelisted players to join");
        let claim_popup_disabled_prop = grid.bool_prop("Disable landclaim notifications");
        let log_privacy_prop =
            grid.enum_prop("Event log privacy:", &["Everybody", "Admins", "Nobody"]);
        let family_share_allowed_prop = grid.bool_prop("Allow family shared accounts");
        let healthbar_distance_prop =
            grid.range_prop("Healthbar visibility distance:", 0.0, 15000.0, 1.0, 1);
        let online_info_visibility_prop = grid.enum_prop(
            "Online player info visibility:",
            &["Show All", "Show Clan", "Show Nobody"],
        );

        let root = root.add(grid.into_inner().end());
        root.group().hide();

        let private_props = include_private.then(|| PrivateProperties {
            motd_prop: motd_prop.unwrap(),
            server_password_prop: server_password_prop.unwrap(),
            admin_password_prop: admin_password_prop.unwrap(),
            pvp_restricted_prop: pvp_restricted_prop.unwrap(),
            pvp_hours_prop: pvp_hours_prop.unwrap(),
            dbd_enabled_prop: dbd_enabled_prop.unwrap(),
            dbd_period_prop: dbd_period_prop.unwrap(),
            no_ownership_prop: no_ownership_prop.unwrap(),
            containers_ignore_ownership_prop: containers_ignore_ownership_prop.unwrap(),
            sandstorm_enabled_prop: sandstorm_enabled_prop.unwrap(),
            clan_markers_enabled_prop: clan_markers_enabled_prop.unwrap(),
            tether_distance_prop: tether_distance_prop.unwrap(),
            max_nudity_prop: max_nudity_prop.unwrap(),
            voice_chat_enabled_prop: voice_chat_enabled_prop.unwrap(),
            enforce_whitelist_prop: enforce_whitelist_prop.unwrap(),
            claim_popup_disabled_prop: claim_popup_disabled_prop.unwrap(),
            log_privacy_prop: log_privacy_prop.unwrap(),
            family_share_allowed_prop: family_share_allowed_prop.unwrap(),
            healthbar_distance_prop: healthbar_distance_prop.unwrap(),
            online_info_visibility_prop: online_info_visibility_prop.unwrap(),
        });

        Rc::new(Self {
            root,
            battleye_required: Cell::new(settings.battleye_required),
            mode_modifier: Cell::new(settings.mode_modifier),
            community: settings.community,
            max_ping: settings.max_ping,
            pvp_enabled_prop,
            raid_enabled_prop,
            raid_restricted_prop,
            raid_hours_prop,
            max_clan_size_prop,
            private_props,
        })
    }

    pub fn set_battleye_required(&self, value: bool) {
        self.battleye_required.set(value);
    }

    pub fn set_mode(&self, value: Mode) {
        self.pvp_enabled_prop
            .clone()
            .set_checked(value != Mode::PVE);
        self.mode_modifier.set(CombatModeModifier::for_mode(value));
    }

    pub fn mode_modifier(&self) -> CombatModeModifier {
        self.mode_modifier.get()
    }

    pub fn set_pvp_enabled_callback(&self, cb: impl Fn(bool) + 'static) {
        self.pvp_enabled_prop
            .clone()
            .set_callback(move |prop| cb(prop.is_checked()));
    }

    pub fn public_values(&self) -> PublicGeneralSettings {
        PublicGeneralSettings {
            battleye_required: self.battleye_required.get(),
            pvp_enabled: self.pvp_enabled_prop.is_checked(),
            mode_modifier: self.mode_modifier.get(),
            community: self.community,
            max_ping: self.max_ping,
            max_clan_size: self.max_clan_size_prop.value().to_u16().unwrap(),
            raid_enabled: self.raid_enabled_prop.is_checked(),
            raid_restricted: self.raid_restricted_prop.is_checked(),
            raid_hours: self.raid_hours_prop.value(),
        }
    }

    pub fn values(&self) -> GeneralSettings {
        self.private_props
            .as_ref()
            .unwrap()
            .values(self.public_values())
    }

    pub fn set_public_values(&self, settings: &PublicGeneralSettings) {
        self.pvp_enabled_prop.set_checked(settings.pvp_enabled);
        self.raid_enabled_prop.set_checked(settings.raid_enabled);
        self.raid_restricted_prop
            .set_checked(settings.raid_restricted);
        self.raid_hours_prop.set_value(&settings.raid_hours);
        self.max_clan_size_prop
            .set_value(settings.max_clan_size as f64);
    }

    pub fn set_values(&self, settings: &GeneralSettings) {
        self.set_public_values(settings);
        self.private_props.as_ref().unwrap().set_values(settings);
    }
}

impl LayoutElement for GeneralTab {
    fn min_size(&self) -> fltk_float::Size {
        self.root.min_size()
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.root.layout(x, y, width, height)
    }
}

impl SettingsTab for GeneralTab {
    fn root(&self) -> impl WidgetExt + 'static {
        self.root.group()
    }
}

impl PrivateProperties {
    fn values(&self, public: PublicGeneralSettings) -> GeneralSettings {
        GeneralSettings {
            public,
            motd: self.motd_prop.value(),
            server_password: self.server_password_prop.value(),
            admin_password: self.admin_password_prop.value(),
            pvp_restricted: self.pvp_restricted_prop.is_checked(),
            pvp_hours: self.pvp_hours_prop.value(),
            dbd_enabled: self.dbd_enabled_prop.is_checked(),
            dbd_period: TimeDelta::try_seconds(self.dbd_period_prop.value() as i64).unwrap(),
            no_ownership: self.no_ownership_prop.is_checked(),
            containers_ignore_ownership: self.containers_ignore_ownership_prop.is_checked(),
            sandstorm_enabled: self.sandstorm_enabled_prop.is_checked(),
            clan_markers_enabled: self.clan_markers_enabled_prop.is_checked(),
            tether_distance: self.tether_distance_prop.value(),
            max_nudity: Nudity::from_repr(self.max_nudity_prop.value() as u8).unwrap(),
            voice_chat_enabled: self.voice_chat_enabled_prop.is_checked(),
            enforce_whitelist: self.enforce_whitelist_prop.is_checked(),
            claim_popup_disabled: self.claim_popup_disabled_prop.is_checked(),
            log_privacy: EventLogPrivacy::from_repr(self.log_privacy_prop.value() as u8).unwrap(),
            family_share_allowed: self.family_share_allowed_prop.is_checked(),
            healthbar_distance: self.healthbar_distance_prop.value(),
            online_info_visibility: OnlinePlayerInfoVisibility::from_repr(
                self.online_info_visibility_prop.value() as u8,
            )
            .unwrap(),
        }
    }

    fn set_values(&self, settings: &GeneralSettings) {
        self.motd_prop.clone().set_value(&settings.motd);
        self.server_password_prop
            .clone()
            .set_value(&settings.server_password);
        self.admin_password_prop
            .clone()
            .set_value(&settings.admin_password);
        self.pvp_restricted_prop
            .set_checked(settings.pvp_restricted);
        self.pvp_hours_prop.set_value(&settings.pvp_hours);
        self.dbd_enabled_prop.set_checked(settings.dbd_enabled);
        self.dbd_period_prop
            .set_value(settings.dbd_period.num_seconds() as f64);
        self.no_ownership_prop.set_checked(settings.no_ownership);
        self.containers_ignore_ownership_prop
            .set_checked(settings.containers_ignore_ownership);
        self.sandstorm_enabled_prop
            .set_checked(settings.sandstorm_enabled);
        self.clan_markers_enabled_prop
            .set_checked(settings.clan_markers_enabled);
        self.tether_distance_prop
            .set_value(settings.tether_distance);
        self.max_nudity_prop.set_value(settings.max_nudity as u8);
        self.voice_chat_enabled_prop
            .set_checked(settings.voice_chat_enabled);
        self.enforce_whitelist_prop
            .set_checked(settings.enforce_whitelist);
        self.claim_popup_disabled_prop
            .set_checked(settings.claim_popup_disabled);
        self.log_privacy_prop.set_value(settings.log_privacy as u8);
        self.family_share_allowed_prop
            .set_checked(settings.family_share_allowed);
        self.healthbar_distance_prop
            .set_value(settings.healthbar_distance);
        self.online_info_visibility_prop
            .set_value(settings.online_info_visibility as u8);
    }
}
