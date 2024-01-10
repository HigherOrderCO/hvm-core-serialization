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

impl From<Book> for hvmc::run::Book {
  fn from(value: Book) -> Self {
    hvmc::ast::book_to_runtime(&value.into())
  }
}

impl From<hvmc::run::Book> for Book {
  fn from(value: hvmc::run::Book) -> Self {
    hvmc::ast::book_from_runtime(&value).into()
  }
}