use crate::net::Net;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Book(BTreeMap<String, Net>);

impl From<Book> for hvm::ast::Book {
  fn from(value: Book) -> Self {
    hvm::ast::Book {
      defs: value.0.into_iter().map(|(k, v)| (k, v.into())).collect(),
    }
  }
}

impl From<hvm::ast::Book> for Book {
  fn from(value: hvm::ast::Book) -> Self {
    Self(value.defs.into_iter().map(|(k, v)| (k, v.into())).collect())
  }
}

// impl From<Book> for hvm::run::Book {
//   fn from(value: Book) -> Self {
//     hvm::ast::book_to_runtime(&value.into())
//   }
// }

// impl From<hvm::run::Book> for Book {
//   fn from(value: hvm::run::Book) -> Self {
//     hvm::ast::book_from_runtime(&value).into()
//   }
// }
