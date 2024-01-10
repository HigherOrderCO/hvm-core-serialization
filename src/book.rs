use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};
use crate::net::Net;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Book(BTreeMap<String, Net>);

impl From<Book> for hvmc::ast::Book {
  fn from(value: Book) -> Self {
    value.0.into_iter().map(|(k, v)| (k, v.into())).collect()
  }
}

impl From<hvmc::ast::Book> for Book {
  fn from(value: hvmc::ast::Book) -> Self {
    Self(value.into_iter().map(|(k, v)| (k, v.into())).collect())
  }
}