use super::scalars::{HVMRef, Tag};
use bitbuffer::{BitRead, BitWrite, Endianness};

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

// Traverse the tree pre-order and writes the tags(and data) of each node
impl<E: Endianness> BitWrite<E> for Tree {
  fn write(&self, stream: &mut bitbuffer::BitWriteStream<E>) -> bitbuffer::Result<()> {
    use hvmc::ast::Tree::*;

    let Tree(node) = self;
    stream.write(&Tag::from(node))?;

    match node {
      Ctr { lft, rgt, .. } | Op2 { lft, rgt } | Mat { sel: lft, ret: rgt } => {
        stream.write(&Tree(lft.as_ref().clone()))?;
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
      branch @ (CON | DUP(_) | OP2 | MAT) => {
        let lft = Box::new(stream.read::<Tree>()?.into());
        let rgt = Box::new(stream.read::<Tree>()?.into());
        match branch {
          CON => Ctr { lab: 0, lft, rgt },
          DUP(lab) => Ctr {
            lab: u8::from(lab) + 1,
            lft,
            rgt,
          },
          OP2 => Op2 { lft, rgt },
          MAT => Mat { sel: lft, ret: rgt },
          _ => unreachable!(),
        }
      }
    };
    Ok(Self(tree))
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
    let cases = ["([* <#123 <#321 *>>] [@a (* *)])", "((@foo *) [* #123])"];
    for tree_source in cases {
      let tree: Tree = do_parse_tree(tree_source).into();

      let bytes = encode(&tree.clone());
      let decoded_tree: Tree = decode(&bytes);
      assert_eq!(tree, decoded_tree, "{}", tree_source);
    }
  }
}
