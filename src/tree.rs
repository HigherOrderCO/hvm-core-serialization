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
      Op { op, rhs, out } => {
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
      Adt {
        lab,
        variant_index,
        variant_count,
        fields,
      } => todo!(),
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
      Ctr { ports, .. } => {
        stream.write::<VarLenNumber>(&(ports.len() as u64).into())?;
        stream.write(&ports.into_iter().map(|p| Tree(p.clone())).collect::<Vec<_>>())?;
        // for port in ports {
        //   stream.write(&Tree(port.clone()))?;
        // }
      }
      Mat { zero, succ, out } => {
        stream.write(&Tree(zero.as_ref().clone()))?;
        stream.write(&Tree(succ.as_ref().clone()))?;
        stream.write(&Tree(out.as_ref().clone()))?;
      }
      Op { rhs, out, .. } => {
        // stream.write(&VarLenNumber::from(*lft))?;
        // stream.write(&Tree(rgt.as_ref().clone()))?;

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
    use hvmc::ops::TypedOp;
    use Tag::*;

    let tag: Tag = stream.read()?;
    let tree = match tag {
      leaf @ (ERA | NUM(_) | REF(_) | VAR) => match leaf {
        ERA => Era,
        REF(HVMRef(nam)) => Ref { nam },
        NUM((false, val)) => Int { val: val.into() },
        NUM((true, val)) => F32 { val: f32::from(val).into() },
        VAR => Var { nam: "invalid".to_string() },
        _ => unreachable!(),
      },
      OPS(opr) => {
        // let lft = stream.read::<VarLenNumber>()?.into();
        let rhs = Box::new(stream.read::<Tree>()?.into());
        let out = Box::new(stream.read::<Tree>()?.into());
        Op {
          rhs,
          out,
          op: TypedOp::try_from(u16::try_from(opr).unwrap()).unwrap(),
        }
      }
      MAT => {
        let zero = Box::new(stream.read::<Tree>()?.into());
        let succ = Box::new(stream.read::<Tree>()?.into());
        let out = Box::new(stream.read::<Tree>()?.into());
        Mat { zero, succ, out }
      }
      CTR((len, lab)) => {
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
    let cases = ["([* (#123 (#321 *))] [@a (* *)])", "((@foo *) [* #123])", "<+ #5 *>", "<+ * *>", "<1+ #5>"];
    for tree_source in cases {
      let tree: Tree = hvmc::ast::Tree::from_str(tree_source).unwrap().into();

      let bytes = encode(&tree.clone());
      let decoded_tree: Tree = decode(&bytes);
      assert_eq!(tree, decoded_tree, "{}", tree_source);
    }
  }
}
