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
      Ctr { ports, .. } => ports
        .iter_mut()
        .map(Self::gather_vars)
        .reduce(|mut acc, mut e| {
          acc.append(&mut e);
          acc
        })
        .unwrap_or_default(),
      Op { fst: rhs, snd: out, .. } => {
        let mut vars = Self::gather_vars(rhs);
        vars.append(&mut Self::gather_vars(out));
        vars
      }
      Mat { zero, succ, out } => {
        let mut vars = Self::gather_vars(zero);
        vars.append(&mut Self::gather_vars(succ));
        vars.append(&mut Self::gather_vars(out));
        vars
      }
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
      Ctr { ref ports, .. } if ports.len() == 2 => {
        stream.write(&Tree(ports[0].clone()))?;
        stream.write(&Tree(ports[1].clone()))?;
      }
      Ctr { ref ports, .. } => {
        stream.write::<VarLenNumber>(&(ports.len() as u64).into())?;
        stream.write(&ports.into_iter().map(|p| Tree(p.clone())).collect::<Vec<_>>())?;
      }
      Mat { zero, succ, out } => {
        stream.write(&Tree(zero.as_ref().clone()))?;
        stream.write(&Tree(succ.as_ref().clone()))?;
        stream.write(&Tree(out.as_ref().clone()))?;
      }
      Op { fst: rhs, snd: out, .. } => {
        stream.write(&Tree(rhs.as_ref().clone()))?;
        stream.write(&Tree(out.as_ref().clone()))?;
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
      leaf @ (NUM(_) | REF(_) | VAR) => match leaf {
        REF(HVMRef(nam)) => Ref { nam },
        NUM(val) => Num { val: val.into() },
        VAR => Var { nam: "invalid".to_string() },
        _ => unreachable!(),
      },
      DynamicCtr((len, _)) if u64::from(len) == 0 => Era,
      OPS => {
        let rhs = Box::new(stream.read::<Tree>()?.into());
        let out = Box::new(stream.read::<Tree>()?.into());
        Op { fst: rhs, snd: out }
      }
      MAT => {
        let zero = Box::new(stream.read::<Tree>()?.into());
        let succ = Box::new(stream.read::<Tree>()?.into());
        let out = Box::new(stream.read::<Tree>()?.into());
        Mat { zero, succ, out }
      }
      StandardCtr(lab) => {
        let lft = Box::new(stream.read::<Tree>()?.into());
        let rgt = Box::new(stream.read::<Tree>()?.into());
        Ctr {
          lab: lab.into(),
          ports: vec![*lft, *rgt],
        }
      }
      DynamicCtr((len, lab)) => {
        let ports: Vec<Tree> = stream.read_sized(u64::from(len) as usize)?;
        Ctr {
          lab: lab.into(),
          ports: ports.into_iter().map(|p| p.0).collect(),
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
  use super::*;
  use std::str::FromStr;

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
      let tree: Tree = hvmc::ast::Tree::from_str(tree_source).unwrap().into();

      let bytes = encode(&tree.clone());
      let decoded_tree: Tree = decode(&bytes);
      assert_eq!(tree, decoded_tree, "{}", tree_source);
    }
  }
}
