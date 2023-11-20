use crate::scalars::VarLenNumber;

use super::tree::Tree;
use super::wiring::Wiring;
use bitbuffer::{BitRead, BitWrite, Endianness};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Net(hvmc::ast::Net);

impl From<Net> for hvmc::ast::Net {
  fn from(value: Net) -> Self {
    value.0
  }
}

impl From<hvmc::ast::Net> for Net {
  fn from(value: hvmc::ast::Net) -> Self {
    Self(value)
  }
}

impl Net {
  fn get_trees(&mut self) -> Vec<&mut hvmc::ast::Tree> {
    let Net(hvmc::ast::Net { root, rdex }) = self;
    let mut trees = vec![root];
    trees.append(&mut rdex.iter_mut().flat_map(|(a, b)| [a, b]).collect::<Vec<_>>());
    trees
  }

  pub fn gather_vars(&mut self) -> Vec<&mut String> {
    fn go(node: &mut hvmc::ast::Tree) -> Vec<&mut String> {
      use hvmc::ast::Tree::*;
      match node {
        Var { nam } => vec![nam],
        Era | Ref { .. } | Num { .. } => vec![],
        Ctr { lft, rgt, .. } | Op2 { lft, rgt } | Mat { sel: lft, ret: rgt } => {
          let mut vars = go(lft);
          vars.append(&mut go(rgt));
          vars
        }
      }
    }
    self.get_trees().into_iter().flat_map(go).collect::<Vec<_>>()
  }

  pub fn apply_wiring(&mut self, wiring: Wiring) {
    let mut vars = self.gather_vars();
    for (i, (a, b)) in wiring.connections.into_iter().enumerate() {
      *vars[a] = i.to_string();
      *vars[b] = i.to_string();
    }
  }

  pub fn get_current_wiring(&self) -> Wiring {
    let mut vars_with_index = self.clone().gather_vars().into_iter().map(|s| s.clone()).enumerate().collect::<Vec<_>>();
    vars_with_index.sort_by(|(_, a_name), (_, b_name)| a_name.cmp(b_name));

    let connections = vars_with_index
      .group_by(|(_, a_name), (_, b_name)| a_name == b_name)
      .map(|v| {
        let [(a_idx, _), (b_idx, _)] = v else { unreachable!() };
        (*a_idx, *b_idx)
      })
      .collect::<Vec<_>>();

    Wiring::new(connections)
  }

  /// Normalizes the net's variable names
  pub fn normalize(mut self) -> Self {
    self.apply_wiring(self.get_current_wiring());
    self
  }
}

impl<E: Endianness> BitWrite<E> for Net {
  fn write(&self, stream: &mut bitbuffer::BitWriteStream<E>) -> bitbuffer::Result<()> {
    let Net(hvmc::ast::Net { root, rdex }) = self;

    stream.write::<Tree>(&root.clone().into())?;
    stream.write::<VarLenNumber>(&(rdex.len() as u32).into())?;
    stream.write(&rdex.iter().map(|(a, b)| (a.clone().into(), b.clone().into())).collect::<Vec<(Tree, Tree)>>())?;
    stream.write(&self.get_current_wiring())?;

    Ok(())
  }
}

impl<E: Endianness> BitRead<'_, E> for Net {
  fn read(stream: &mut bitbuffer::BitReadStream<'_, E>) -> bitbuffer::Result<Self> {
    let root: Tree = stream.read()?;
    let rdex_len: u32 = stream.read::<VarLenNumber>()?.into();
    let rdex: Vec<(Tree, Tree)> = stream.read_sized(rdex_len as usize)?;

    let mut net: Net = hvmc::ast::Net {
      root: root.into(),
      rdex: rdex.into_iter().map(|(a, b)| (a.into(), b.into())).collect::<Vec<_>>(),
    }
    .into();

    let wiring: Wiring = stream.read_sized(net.gather_vars().len() / 2)?;

    net.apply_wiring(wiring);

    Ok(net)
  }
}

#[cfg(test)]
mod tests {
  use super::super::{decode, encode};
  use super::*;
  use hvmc::ast::do_parse_net;

  #[test]
  fn test_net_encoding() {
    let cases = [
      "*",
      "* & * ~ *",
      "(a a)",
      "a & * ~ (b R) & (b a) ~ (R *)",
      "(a (a b)) & (b *) ~ (c c)",
      "((a [a b]) b)", // Y-Combinator
    ];
    for net in cases {
      let net: Net = do_parse_net(net).into();
      let net_string = format!("{:?}", net);

      let bytes = encode(&net);
      let decoded_net: Net = decode(&bytes);

      assert_eq!(net.normalize(), decoded_net.normalize(), "{}", net_string);
    }
  }
}
