mod filter;
mod order;

pub use filter::{Filter, TypeFilter};
pub use order::{SortCriteria, SortKey};

use crate::gui::data::TableView;
use crate::servers::Server;

pub type ServerBrowserState = TableView<Vec<Server>, Filter, SortCriteria>;
