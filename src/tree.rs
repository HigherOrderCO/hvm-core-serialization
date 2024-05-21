use super::scalars::{HVMRef, Tag};
use crate::{decode, encode};
use bitbuffer::{BitRead, BitWrite, Endianness};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Tree(hvm::ast::Tree);

impl From<Tree> for hvm::ast::Tree {
  fn from(value: Tree) -> Self {
    value.0
  }
}
impl From<hvm::ast::Tree> for Tree {
  fn from(value: hvm::ast::Tree) -> Self {
    Self(value)
  }
}

impl Tree {
  pub fn gather_vars(tree: &mut hvm::ast::Tree) -> Vec<&mut String> {
    use hvm::ast::Tree::*;
    match tree {
      Var { nam } => vec![nam],
      Con { fst, snd } | Dup { fst, snd } | Swi { fst, snd } | Opr { fst, snd } => {
        let mut vars = Self::gather_vars(fst);
        vars.append(&mut Self::gather_vars(snd));
        vars
      }
      _ => vec![],
    }
  }
}

// Traverse the tree pre-order and writes the tags(and data) of each node
impl<E: Endianness> BitWrite<E> for Tree {
  fn write(&self, stream: &mut bitbuffer::BitWriteStream<E>) -> bitbuffer::Result<()> {
    use hvm::ast::Tree::*;

    let Tree(node) = self;
    stream.write(&Tag::from(node))?;

    match node {
      Con { fst, snd } | Dup { fst, snd } | Swi { fst, snd } | Opr { fst, snd } => {
        stream.write(&Tree(fst.as_ref().clone()))?;
        stream.write(&Tree(snd.as_ref().clone()))?;
      }
      _ => {}
    }

    Ok(())
  }
}

impl<E: Endianness> BitRead<'_, E> for Tree {
  fn read(stream: &mut bitbuffer::BitReadStream<'_, E>) -> bitbuffer::Result<Self> {
    use hvm::ast::Tree::*;
    use Tag::*;

    let tag: Tag = stream.read()?;
    let tree = match tag {
      leaf @ (NUM(_) | REF(_) | VAR) => match leaf {
        REF(HVMRef(nam)) => Ref { nam },
        NUM(val) => Num {
          val: hvm::ast::Numb(val.into()),
        },
        VAR => Var { nam: "invalid".to_string() },
        _ => unreachable!(),
      },
      ERA => Era,
      OPS => {
        let rhs = Box::new(stream.read::<Tree>()?.into());
        let out = Box::new(stream.read::<Tree>()?.into());
        Opr { fst: rhs, snd: out }
      }
      MAT => {
        let fst = Box::new(stream.read::<Tree>()?.into());
        let snd = Box::new(stream.read::<Tree>()?.into());
        Swi { fst, snd }
      }
      CON => {
        let fst = Box::new(stream.read::<Tree>()?.into());
        let snd = Box::new(stream.read::<Tree>()?.into());
        Con { fst, snd }
      }
      DUP => {
        let fst = Box::new(stream.read::<Tree>()?.into());
        let snd = Box::new(stream.read::<Tree>()?.into());
        Dup { fst, snd }
      }
    };
    Ok(Self(tree))
  }
}

impl Serialize for Tree {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_bytes(&encode(self))
  }
}

impl<'de> Deserialize<'de> for Tree {
  fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let bytes: Vec<u8> = serde::Deserialize::deserialize(deserializer)?;
    Ok(decode(&bytes))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use hvm::ast::CoreParser;

  // Tree-only encoding does not support variables
  #[test]
  fn test_tree_encoding() {
    let cases = [
      "(* *)",
      "({* *} *)",
      "({* (123 (321 *))} {@a (* *)})",
      "((@foo *) {* 123})",
      "$(+ 5 *)",
      // "$(+ * *)",
      "$(- 5 3)",
      "$(+ 5.5 *)",
    ];
    for tree_source in cases {
      let tree: Tree = CoreParser::new(tree_source).parse_tree().unwrap().into();

      let bytes = encode(&tree.clone());
      let decoded_tree: Tree = decode(&bytes);
      assert_eq!(tree, decoded_tree, "{}", tree_source);
    }
  }
}
