use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::BitXor;
use std::rc::Rc;
use std::str::FromStr;

use anyhow::Result;
use fltk::button::{Button, CheckButton, ReturnButton};
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::input::Input;
use fltk::prelude::*;
use fltk::window::Window;
use fltk_float::grid::{Grid, GridBuilder};
use fltk_float::overlay::Overlay;
use fltk_float::{LayoutElement, WrapperFactory};
use ini_persist::load::ParseProperty;
use ini_persist::save::DisplayProperty;
use strum::IntoEnumIterator;
use strum_macros::FromRepr;

use crate::game::settings::server::{Community, DropOnDeath};
use crate::game::settings::Multiplier;
use crate::gui::prelude::WidgetConvenienceExt;
use crate::gui::widgets::DropDownList;
use crate::gui::{alert_error, min_input_width, wrapper_factory};
use crate::servers::{EnumFilter, RangeFilter};
use crate::util::weak_cb;

use super::community_name;
use super::filter_pane::FilterHolder;

pub struct AdvancedFilterDialog<F: FilterHolder + 'static> {
    filter_holder: Rc<F>,
    window: Window,
    community_input: EnumFilterInput<Community>,
    max_clan_size_input: RangeFilterInput<u16>,
    raid_enabled_input: BoolFilterInput,
    raid_restricted_input: BoolFilterInput,
    xp_rate_mult_input: RangeFilterInput<Multiplier>,
    day_cycle_speed_mult_input: RangeFilterInput<Multiplier>,
    dawn_dusk_speed_mult_input: RangeFilterInput<Multiplier>,
    use_catch_up_time_input: BoolFilterInput,
    stamina_cost_mult_input: RangeFilterInput<Multiplier>,
    active_thirst_mult_input: RangeFilterInput<Multiplier>,
    active_hunger_mult_input: RangeFilterInput<Multiplier>,
    idle_thirst_mult_input: RangeFilterInput<Multiplier>,
    idle_hunger_mult_input: RangeFilterInput<Multiplier>,
    drop_items_on_death_input: EnumFilterInput<DropOnDeath>,
    anyone_can_loot_corpse_input: BoolFilterInput,
    durability_mult_input: RangeFilterInput<Multiplier>,
    thrall_wakeup_time_input: RangeFilterInput<i64>,
    item_spoil_rate_mult_input: RangeFilterInput<Multiplier>,
    harvest_amount_mult_input: RangeFilterInput<Multiplier>,
    rsrc_respawn_speed_mult_input: RangeFilterInput<Multiplier>,
    crafting_time_mult_input: RangeFilterInput<Multiplier>,
    thrall_crafting_time_mult_input: RangeFilterInput<Multiplier>,
}

impl<F: FilterHolder + 'static> AdvancedFilterDialog<F> {
    pub fn new(parent: &impl WindowExt, filter_holder: Rc<F>) -> Rc<Self> {
        let mut window = GridBuilder::with_factory(
            Window::default()
                .with_size(800, 600)
                .with_label("Server Settings"),
            wrapper_factory(),
        )
        .with_padding(10, 10, 10, 10)
        .with_col_spacing(10)
        .with_row_spacing(5);

        window.col().add();
        window.col().add();
        window.col().add();

        let community_input = EnumFilterInput::new(&mut window, "Community", community_name);
        let max_clan_size_input = RangeFilterInput::new(&mut window, "Clan max size");
        let raid_enabled_input = BoolFilterInput::new(&mut window, "PVP building damage enabled");
        let raid_restricted_input =
            BoolFilterInput::new(&mut window, "Time restrict building damage");
        let xp_rate_mult_input = RangeFilterInput::new(&mut window, "Player XP rate multiplier");
        let day_cycle_speed_mult_input = RangeFilterInput::new(&mut window, "Day cycle speed");
        let dawn_dusk_speed_mult_input = RangeFilterInput::new(&mut window, "Dawn/dusk time speed");
        let use_catch_up_time_input = BoolFilterInput::new(&mut window, "Use catch up time");
        let stamina_cost_mult_input = RangeFilterInput::new(&mut window, "Stamina cost multiplier");
        let active_thirst_mult_input =
            RangeFilterInput::new(&mut window, "Player active thirst multiplier");
        let active_hunger_mult_input =
            RangeFilterInput::new(&mut window, "Player active hunger multiplier");
        let idle_thirst_mult_input =
            RangeFilterInput::new(&mut window, "Player idle thirst multiplier");
        let idle_hunger_mult_input =
            RangeFilterInput::new(&mut window, "Player idle hunger multiplier");
        let drop_items_on_death_input = EnumFilterInput::new(
            &mut window,
            "Equipment dropped on death",
            drop_on_death_name,
        );
        let anyone_can_loot_corpse_input =
            BoolFilterInput::new(&mut window, "Everybody can loot corpse");
        let durability_mult_input = RangeFilterInput::new(&mut window, "Durability multiplier");
        let thrall_wakeup_time_input =
            RangeFilterInput::new(&mut window, "Thrall wakeup time (seconds)");
        let item_spoil_rate_mult_input =
            RangeFilterInput::new(&mut window, "Item spoil rate scale");
        let harvest_amount_mult_input =
            RangeFilterInput::new(&mut window, "Harvest amount multiplier");
        let rsrc_respawn_speed_mult_input =
            RangeFilterInput::new(&mut window, "Resource respawn speed multiplier");
        let crafting_time_mult_input =
            RangeFilterInput::new(&mut window, "Crafting time multiplier");
        let thrall_crafting_time_mult_input =
            RangeFilterInput::new(&mut window, "Thrall crafting time multiplier");

        window.row().add();
        let mut actions = Grid::builder_with_factory(wrapper_factory())
            .with_col_spacing(10)
            .with_top_padding(5);
        actions.row().add();
        let col_group = actions.col_group().add();
        actions.col().with_stretch(1).add();
        actions.extend_group(col_group).batch(2);
        actions.cell().unwrap().skip();
        let mut apply_button = actions
            .cell()
            .unwrap()
            .wrap(ReturnButton::default().with_label("Apply"));
        let mut cancel_button = actions
            .cell()
            .unwrap()
            .wrap(Button::default().with_label("Cancel"));
        window.span(1, 3).unwrap().add(actions.end());

        let window_grid = window.end();
        let window_size = window_grid.min_size();
        let mut window = window_grid.group();
        window.set_size(window_size.width, window_size.height);
        window_grid.layout_children();

        window.set_pos(
            parent.x() + (parent.w() - window.w()) / 2,
            parent.y() + (parent.h() - window.h()) / 2,
        );

        filter_holder.access_filter(|filter| {
            community_input.set_value(&filter.community);
            max_clan_size_input.set_value(&filter.max_clan_size);
            raid_enabled_input.set_value(&filter.raid_enabled);
            raid_restricted_input.set_value(&filter.raid_restricted);
            xp_rate_mult_input.set_value(&filter.xp_rate_mult);
            day_cycle_speed_mult_input.set_value(&filter.day_cycle_speed_mult);
            dawn_dusk_speed_mult_input.set_value(&filter.dawn_dusk_speed_mult);
            use_catch_up_time_input.set_value(&filter.use_catch_up_time);
            stamina_cost_mult_input.set_value(&filter.stamina_cost_mult);
            active_thirst_mult_input.set_value(&filter.active_thirst_mult);
            active_hunger_mult_input.set_value(&filter.active_hunger_mult);
            idle_thirst_mult_input.set_value(&filter.idle_thirst_mult);
            idle_hunger_mult_input.set_value(&filter.idle_hunger_mult);
            drop_items_on_death_input.set_value(&filter.drop_items_on_death);
            anyone_can_loot_corpse_input.set_value(&filter.anyone_can_loot_corpse);
            durability_mult_input.set_value(&filter.durability_mult);
            thrall_wakeup_time_input.set_value(&filter.thrall_wakeup_time_secs);
            item_spoil_rate_mult_input.set_value(&filter.item_spoil_rate_mult);
            harvest_amount_mult_input.set_value(&filter.harvest_amount_mult);
            rsrc_respawn_speed_mult_input.set_value(&filter.rsrc_respawn_speed_mult);
            crafting_time_mult_input.set_value(&filter.crafting_time_mult);
            thrall_crafting_time_mult_input.set_value(&filter.thrall_crafting_time_mult);
        });

        let this = Rc::new(Self {
            filter_holder,
            window,
            community_input,
            max_clan_size_input,
            raid_enabled_input,
            raid_restricted_input,
            xp_rate_mult_input,
            day_cycle_speed_mult_input,
            dawn_dusk_speed_mult_input,
            use_catch_up_time_input,
            stamina_cost_mult_input,
            active_thirst_mult_input,
            active_hunger_mult_input,
            idle_thirst_mult_input,
            idle_hunger_mult_input,
            drop_items_on_death_input,
            anyone_can_loot_corpse_input,
            durability_mult_input,
            thrall_wakeup_time_input,
            item_spoil_rate_mult_input,
            harvest_amount_mult_input,
            rsrc_respawn_speed_mult_input,
            crafting_time_mult_input,
            thrall_crafting_time_mult_input,
        });

        apply_button.set_callback(weak_cb!([this] => |_| this.apply_clicked()));
        cancel_button.set_callback(weak_cb!([this] => |_| this.cancel_clicked()));

        this
    }

    pub fn run(&self) {
        let mut window = self.window.clone();
        window.make_modal(true);
        window.show();

        while window.shown() && !fltk::app::should_program_quit() {
            fltk::app::wait();
        }
    }

    fn apply_clicked(&self) {
        if self.apply_changes().is_ok() {
            self.window.clone().hide();
        }
    }

    fn cancel_clicked(&self) {
        self.window.clone().hide();
    }

    fn apply_changes(&self) -> Result<()> {
        let community = self.community_input.value();
        let max_clan_size = self.max_clan_size_input.value()?;
        let raid_enabled = self.raid_enabled_input.value();
        let raid_restricted = self.raid_restricted_input.value();
        let xp_rate_mult = self.xp_rate_mult_input.value()?;
        let day_cycle_speed_mult = self.day_cycle_speed_mult_input.value()?;
        let dawn_dusk_speed_mult = self.dawn_dusk_speed_mult_input.value()?;
        let use_catch_up_time = self.use_catch_up_time_input.value();
        let stamina_cost_mult = self.stamina_cost_mult_input.value()?;
        let active_thirst_mult = self.active_thirst_mult_input.value()?;
        let active_hunger_mult = self.active_hunger_mult_input.value()?;
        let idle_thirst_mult = self.idle_thirst_mult_input.value()?;
        let idle_hunger_mult = self.idle_hunger_mult_input.value()?;
        let drop_items_on_death = self.drop_items_on_death_input.value();
        let anyone_can_loot_corpse = self.anyone_can_loot_corpse_input.value();
        let durability_mult = self.durability_mult_input.value()?;
        let thrall_wakeup_time = self.thrall_wakeup_time_input.value()?;
        let item_spoil_rate_mult = self.item_spoil_rate_mult_input.value()?;
        let harvest_amount_mult = self.harvest_amount_mult_input.value()?;
        let rsrc_respawn_speed_mult = self.rsrc_respawn_speed_mult_input.value()?;
        let crafting_time_mult = self.crafting_time_mult_input.value()?;
        let thrall_crafting_time_mult = self.thrall_crafting_time_mult_input.value()?;

        self.filter_holder.mutate_filter(move |filter| {
            filter.community = community;
            filter.max_clan_size = max_clan_size;
            filter.raid_enabled = raid_enabled;
            filter.raid_restricted = raid_restricted;
            filter.xp_rate_mult = xp_rate_mult;
            filter.day_cycle_speed_mult = day_cycle_speed_mult;
            filter.dawn_dusk_speed_mult = dawn_dusk_speed_mult;
            filter.use_catch_up_time = use_catch_up_time;
            filter.stamina_cost_mult = stamina_cost_mult;
            filter.active_thirst_mult = active_thirst_mult;
            filter.active_hunger_mult = active_hunger_mult;
            filter.idle_thirst_mult = idle_thirst_mult;
            filter.idle_hunger_mult = idle_hunger_mult;
            filter.drop_items_on_death = drop_items_on_death;
            filter.anyone_can_loot_corpse = anyone_can_loot_corpse;
            filter.durability_mult = durability_mult;
            filter.thrall_wakeup_time_secs = thrall_wakeup_time;
            filter.item_spoil_rate_mult = item_spoil_rate_mult;
            filter.harvest_amount_mult = harvest_amount_mult;
            filter.rsrc_respawn_speed_mult = rsrc_respawn_speed_mult;
            filter.crafting_time_mult = crafting_time_mult;
            filter.thrall_crafting_time_mult = thrall_crafting_time_mult;
        });

        Ok(())
    }
}

struct BoolFilterInput {
    active_check: CheckButton,
    value_input: DropDownList,
}

impl BoolFilterInput {
    pub fn new<G: GroupExt + Clone, F: Borrow<WrapperFactory>>(
        grid: &mut GridBuilder<G, F>,
        label: &str,
    ) -> Self {
        grid.row().add();

        let mut active_check = grid
            .cell()
            .unwrap()
            .wrap(CheckButton::default())
            .with_label(label);

        grid.cell().unwrap().skip();

        let mut value_input = grid.cell().unwrap().wrap(DropDownList::default());
        value_input.add("Yes");
        value_input.add("No");
        value_input.set_activated(false);

        active_check.set_callback({
            let mut value_input = value_input.clone();
            move |check| {
                let checked = check.is_checked();

                value_input.set_activated(checked);

                if checked {
                    value_input.set_value(0);
                } else {
                    value_input.set_value(-1);
                }
            }
        });

        Self {
            active_check,
            value_input,
        }
    }

    pub fn value(&self) -> Option<bool> {
        self.active_check
            .is_checked()
            .then(|| self.value_input.value() == 0)
    }

    pub fn set_value(&self, filter: &Option<bool>) {
        let mut value_input = self.value_input.clone();
        self.active_check.set_checked(filter.is_some());
        value_input.set_activated(filter.is_some());
        value_input.set_value(match filter {
            None => -1,
            Some(true) => 0,
            Some(false) => 1,
        });
    }
}

#[derive(Debug, Clone, Copy, FromRepr)]
#[repr(i32)]
enum FilterOp {
    EQ,
    NE,
    LT,
    LE,
    GT,
    GE,
    IN,
    OUT,
}

impl BitXor<bool> for FilterOp {
    type Output = Self;
    fn bitxor(self, rhs: bool) -> Self::Output {
        if !rhs {
            return self;
        }
        match self {
            Self::EQ => Self::NE,
            Self::NE => Self::EQ,
            Self::LT => Self::GE,
            Self::LE => Self::GT,
            Self::GT => Self::LE,
            Self::GE => Self::LT,
            Self::IN => Self::OUT,
            Self::OUT => Self::IN,
        }
    }
}

struct RangeFilterInput<T: ParseProperty + DisplayProperty + Copy + PartialOrd> {
    active_check: CheckButton,
    op_input: DropDownList,
    value_input: Input,
    range_group: Group,
    min_input: Input,
    max_input: Input,
    _phantom: PhantomData<T>,
}

impl<T: ParseProperty + DisplayProperty + Copy + PartialOrd> RangeFilterInput<T> {
    pub fn new<G: GroupExt + Clone, F: Borrow<WrapperFactory>>(
        grid: &mut GridBuilder<G, F>,
        label: &str,
    ) -> Self {
        let input_width = min_input_width(&["999999"]);

        grid.row().add();

        let mut active_check = grid
            .cell()
            .unwrap()
            .wrap(CheckButton::default())
            .with_label(label);

        let mut op_input = grid.cell().unwrap().wrap(DropDownList::default());
        op_input.add("is");
        op_input.add("is not");
        op_input.add("is less than");
        op_input.add("is at most");
        op_input.add("is greater than");
        op_input.add("is at least");
        op_input.add("is between");
        op_input.add("is not between");

        let mut value_overlay = Overlay::builder_with_factory(wrapper_factory());

        let mut value_input = value_overlay.wrap(Input::default());

        let mut range_grid = Grid::builder_with_factory(wrapper_factory()).with_col_spacing(5);
        range_grid.row().add();
        range_grid
            .col()
            .with_min_size(input_width)
            .with_stretch(1)
            .add();
        range_grid.col().add();
        range_grid
            .col()
            .with_min_size(input_width)
            .with_stretch(1)
            .add();
        let mut min_input = range_grid.cell().unwrap().wrap(Input::default());
        range_grid
            .cell()
            .unwrap()
            .wrap(Frame::default())
            .with_label("and");
        let mut max_input = range_grid.cell().unwrap().wrap(Input::default());

        let range_grid = range_grid.end();
        let mut range_group = range_grid.group();
        range_group.hide();

        value_overlay.add(range_grid);

        grid.cell().unwrap().add(value_overlay.end());

        op_input.set_activated(false);
        value_input.set_activated(false);
        min_input.set_activated(false);
        max_input.set_activated(false);

        active_check.set_callback({
            let mut op_input = op_input.clone();
            let mut value_input = value_input.clone();
            let mut range_group = range_group.clone();
            let mut min_input = min_input.clone();
            let mut max_input = max_input.clone();
            move |check| {
                let checked = check.is_checked();

                op_input.set_activated(checked);
                value_input.set_activated(checked);
                min_input.set_activated(checked);
                max_input.set_activated(checked);

                if checked {
                    op_input.set_value(0);
                } else {
                    op_input.set_value(-1);
                    value_input.set_value("");
                    value_input.show();
                    range_group.hide();
                }
            }
        });

        op_input.set_callback({
            let mut value_input = value_input.clone();
            let mut range_group = range_group.clone();
            let mut min_input = min_input.clone();
            let mut max_input = max_input.clone();
            move |input| {
                let Some(op) = FilterOp::from_repr(input.value()) else {
                    return;
                };
                if let FilterOp::IN | FilterOp::OUT = op {
                    if value_input.visible() {
                        min_input.set_value("");
                        max_input.set_value("");
                        value_input.hide();
                        range_group.show();
                    }
                } else {
                    if range_group.visible() {
                        value_input.set_value("");
                        value_input.show();
                        range_group.hide();
                    }
                }
            }
        });

        Self {
            active_check,
            op_input,
            value_input,
            range_group,
            min_input,
            max_input,
            _phantom: PhantomData,
        }
    }

    pub fn value(&self) -> Result<Option<RangeFilter<T>>> {
        let result = self.try_value();
        if let Err(err) = result.as_ref() {
            alert_error(
                &format!("{} has an invalid value", self.active_check.label()),
                err,
            );
        }
        result
    }

    fn try_value(&self) -> Result<Option<RangeFilter<T>>> {
        if !self.active_check.is_checked() {
            return Ok(None);
        }
        let op = FilterOp::from_repr(self.op_input.value()).unwrap();
        Ok(match op {
            FilterOp::EQ => {
                let value = T::parse(&self.value_input.value())?;
                Some(RangeFilter {
                    min: Some(value),
                    max: Some(value),
                    negate: false,
                })
            }
            FilterOp::NE => {
                let value = T::parse(&self.value_input.value())?;
                Some(RangeFilter {
                    min: Some(value),
                    max: Some(value),
                    negate: true,
                })
            }
            FilterOp::LT => {
                let value = T::parse(&self.value_input.value())?;
                Some(RangeFilter {
                    min: Some(value),
                    max: None,
                    negate: true,
                })
            }
            FilterOp::LE => {
                let value = T::parse(&self.value_input.value())?;
                Some(RangeFilter {
                    min: None,
                    max: Some(value),
                    negate: false,
                })
            }
            FilterOp::GT => {
                let value = T::parse(&self.value_input.value())?;
                Some(RangeFilter {
                    min: None,
                    max: Some(value),
                    negate: true,
                })
            }
            FilterOp::GE => {
                let value = T::parse(&self.value_input.value())?;
                Some(RangeFilter {
                    min: Some(value),
                    max: None,
                    negate: false,
                })
            }
            FilterOp::IN => {
                let min = T::parse(&self.min_input.value())?;
                let max = T::parse(&self.max_input.value())?;
                Some(RangeFilter {
                    min: Some(min),
                    max: Some(max),
                    negate: false,
                })
            }
            FilterOp::OUT => {
                let min = T::parse(&self.min_input.value())?;
                let max = T::parse(&self.max_input.value())?;
                Some(RangeFilter {
                    min: Some(min),
                    max: Some(max),
                    negate: false,
                })
            }
        })
    }

    pub fn set_value(&self, filter: &Option<RangeFilter<T>>) {
        let mut op_input = self.op_input.clone();
        let mut value_input = self.value_input.clone();
        let mut range_group = self.range_group.clone();
        let mut min_input = self.min_input.clone();
        let mut max_input = self.max_input.clone();

        self.active_check.set_checked(filter.is_some());
        op_input.set_activated(filter.is_some());
        value_input.set_activated(filter.is_some());
        min_input.set_activated(filter.is_some());
        max_input.set_activated(filter.is_some());

        let (op, value, range) = match filter {
            None => (None, None, None),
            Some(RangeFilter {
                min: Some(min),
                max: Some(max),
                negate,
            }) if min == max => (Some(FilterOp::EQ ^ *negate), Some(*min), None),
            Some(RangeFilter {
                min: Some(min),
                max: Some(max),
                negate,
            }) => (Some(FilterOp::IN ^ *negate), None, Some((*min, *max))),
            Some(RangeFilter {
                min: Some(min),
                max: None,
                negate,
            }) => (Some(FilterOp::GE ^ *negate), Some(*min), None),
            Some(RangeFilter {
                min: None,
                max: Some(max),
                negate,
            }) => (Some(FilterOp::LE ^ *negate), Some(*max), None),
            Some(RangeFilter {
                min: None,
                max: None,
                ..
            }) => (None, None, None),
        };

        op_input.set_value(op.map(|op| op as i32).unwrap_or(-1));
        value_input.set_value(&value.as_ref().map(T::display).unwrap_or_default());
        match range {
            None => {
                value_input.show();
                range_group.hide();
            }
            Some((min, max)) => {
                min_input.set_value(&min.display());
                max_input.set_value(&max.display());
                value_input.hide();
                range_group.show();
            }
        }
    }
}

struct EnumFilterInput<T: FromStr + Into<&'static str> + IntoEnumIterator + Copy + Eq> {
    active_check: CheckButton,
    op_input: DropDownList,
    value_input: DropDownList,
    _phantom: PhantomData<T>,
}

impl<T: std::fmt::Debug + FromStr + Into<&'static str> + IntoEnumIterator + Copy + Eq>
    EnumFilterInput<T>
{
    pub fn new<G: GroupExt + Clone, F: Borrow<WrapperFactory>>(
        grid: &mut GridBuilder<G, F>,
        label: &str,
        variant_renderer: impl Fn(T) -> &'static str,
    ) -> Self {
        grid.row().add();

        let mut active_check = grid
            .cell()
            .unwrap()
            .wrap(CheckButton::default())
            .with_label(label);

        let mut op_input = grid.cell().unwrap().wrap(DropDownList::default());
        op_input.add("is");
        op_input.add("is not");

        let mut value_input = grid.cell().unwrap().wrap(DropDownList::default());
        for variant in T::iter() {
            value_input.add(variant_renderer(variant));
        }

        op_input.set_activated(false);
        value_input.set_activated(false);

        active_check.set_callback({
            let mut op_input = op_input.clone();
            let mut value_input = value_input.clone();
            move |check| {
                let checked = check.is_checked();

                op_input.set_activated(checked);
                value_input.set_activated(checked);

                if checked {
                    op_input.set_value(0);
                    value_input.set_value(0);
                } else {
                    op_input.set_value(-1);
                    value_input.set_value(-1);
                }
            }
        });

        Self {
            active_check,
            op_input,
            value_input,
            _phantom: PhantomData,
        }
    }

    pub fn value(&self) -> Option<EnumFilter<T>> {
        self.active_check.is_checked().then(|| {
            let value = T::iter().nth(self.value_input.value() as usize).unwrap();
            let negate = self.op_input.value() != 0;
            EnumFilter { value, negate }
        })
    }

    pub fn set_value(&self, filter: &Option<EnumFilter<T>>) {
        let mut op_input = self.op_input.clone();
        let mut value_input = self.value_input.clone();
        self.active_check.set_checked(filter.is_some());
        op_input.set_activated(filter.is_some());
        value_input.set_activated(filter.is_some());
        match filter {
            None => {
                op_input.set_value(-1);
                value_input.set_value(-1);
            }
            Some(EnumFilter { value, negate }) => {
                op_input.set_value(if *negate { 1 } else { 0 });

                // Yes, this is inefficient, but there's no trait to convert an enum into its repr
                // and I really don't want to pass another lambda into this whole thing ¯\_(ツ)_/¯
                value_input.set_value(T::iter().position(|v| v == *value).unwrap() as i32);
            }
        }
    }
}

fn drop_on_death_name(variant: DropOnDeath) -> &'static str {
    match variant {
        DropOnDeath::Nothing => "Nothing",
        DropOnDeath::All => "Everything",
        DropOnDeath::Backpack => "Backpack Only",
    }
}
