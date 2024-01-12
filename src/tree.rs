use super::scalars::{HVMRef, Tag};
use crate::scalars::VarLenNumber;
use crate::{decode, encode};
use bitbuffer::{BitRead, BitWrite, Endianness};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Tree(hvmc::ast::Tree);

impl From<Tree> for hvmc::ast::Tree {
  fn from(value: Tree) -> Self {
    value.0
  }
}
impl From<hvmc::ast::Tree> for Tree {
  fn from(value: hvmc::ast::Tree) -> Self {
    Self(value)
  }
}

impl Tree {
  pub fn gather_vars(tree: &mut hvmc::ast::Tree) -> Vec<&mut String> {
    use hvmc::ast::Tree::*;
    match tree {
      Var { nam } => vec![nam],
      Con { lft, rgt } | Tup { lft, rgt } | Dup { lft, rgt, .. } | Op2 { lft, rgt, .. } | Mat { sel: lft, ret: rgt } => {
        let mut vars = Self::gather_vars(lft);
        vars.append(&mut Self::gather_vars(rgt));
        vars
      }
      Op1 { rgt, .. } => Self::gather_vars(rgt),
      _ => vec![],
    }
  }
}

// Traverse the tree pre-order and writes the tags(and data) of each node
impl<E: Endianness> BitWrite<E> for Tree {
  fn write(&self, stream: &mut bitbuffer::BitWriteStream<E>) -> bitbuffer::Result<()> {
    use hvmc::ast::Tree::*;

    let Tree(node) = self;
    stream.write(&Tag::from(node))?;

    match node {
      Con { lft, rgt } | Tup { lft, rgt } | Dup { lft, rgt, .. } | Op2 { lft, rgt, .. } | Mat { sel: lft, ret: rgt } => {
        stream.write(&Tree(lft.as_ref().clone()))?;
        stream.write(&Tree(rgt.as_ref().clone()))?;
      }
      Op1 { lft, rgt, .. } => {
        stream.write(&VarLenNumber::from(*lft))?;
        stream.write(&Tree(rgt.as_ref().clone()))?;
      }
      _ => {}
    }

    Ok(())
  }
}

impl<E: Endianness> BitRead<'_, E> for Tree {
  fn read(stream: &mut bitbuffer::BitReadStream<'_, E>) -> bitbuffer::Result<Self> {
    use hvmc::ast::Tree::*;
    use Tag::*;

    let tag: Tag = stream.read()?;
    let tree = match tag {
      leaf @ (ERA | NUM(_) | REF(_) | VAR) => match leaf {
        ERA => Era,
        REF(HVMRef(nam)) => Ref { nam },
        NUM(val) => Num { val: val.into() },
        VAR => Var { nam: "invalid".to_string() },
        _ => unreachable!(),
      },
      OPS(opr) if (opr >> 4) == 1 => {
        let lft = stream.read::<VarLenNumber>()?.into();
        let rgt = Box::new(stream.read::<Tree>()?.into());
        Op1 { lft, rgt, opr: opr & 0b01111 }
      }
      branch @ (CON | DUP(_) | OPS(_) | MAT) => {
        let lft = Box::new(stream.read::<Tree>()?.into());
        let rgt = Box::new(stream.read::<Tree>()?.into());
        match branch {
          CON => Con { lft, rgt },
          DUP(lab) if u8::from(lab) == 0 => Tup { lft, rgt },
          DUP(lab) => Dup {
            lab: (u8::from(lab) + 1) as u32,
            lft,
            rgt,
          },
          OPS(opr) => Op2 { lft, rgt, opr },
          MAT => Mat { sel: lft, ret: rgt },
          _ => unreachable!(),
        }
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
  use super::super::{decode, encode};
  use super::*;
  use hvmc::ast::do_parse_tree;

  // Tree-only encoding does not support variables
  #[test]
  fn test_tree_encoding() {
    let cases = ["([* (#123 (#321 *))] [@a (* *)])", "((@foo *) [* #123])", "<+ #5 *>", "<+ * *>", "<1+ #5>"];
    for tree_source in cases {
      let tree: Tree = do_parse_tree(tree_source).into();

      let bytes = encode(&tree.clone());
      let decoded_tree: Tree = decode(&bytes);
      assert_eq!(tree, decoded_tree, "{}", tree_source);
    }
  }
}
