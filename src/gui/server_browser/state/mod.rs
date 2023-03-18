mod filter;
mod order;

pub use filter::Filter;
pub use order::SortOrder;

use crate::gui::data::TableView;
use crate::servers::Server;

pub type ServerBrowserState = TableView<Vec<Server>, Filter, SortOrder>;
