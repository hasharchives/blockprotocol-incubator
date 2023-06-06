mod boolean;
mod empty_list;
mod null;
mod number;
mod object;
mod text;

pub use boolean::Boolean;
pub use empty_list::EmptyList;
pub use null::Null;
pub use number::Number;
pub use object::Object;
pub use text::Text;

use crate::path::EntityPath;

pub struct DataTypePath;

impl From<DataTypePath> for EntityPath<'static> {
    fn from(_: DataTypePath) -> Self {
        EntityPath::new(&[])
    }
}
