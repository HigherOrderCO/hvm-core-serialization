use super::tree::Tree;
use super::wiring::Wiring;
use crate::scalars::VarLenNumber;
use crate::{decode, encode};
use bitbuffer::{BitRead, BitWrite, Endianness};
use serde::{Deserialize, Serialize};

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
    let Net(hvmc::ast::Net { root, redexes }) = self;
    let mut trees = vec![root];
    trees.append(&mut redexes.iter_mut().flat_map(|(a, b)| [a, b]).collect::<Vec<_>>());
    trees
  }

  pub fn gather_vars(&mut self) -> Vec<&mut String> {
    self.get_trees().into_iter().flat_map(Tree::gather_vars).collect::<Vec<_>>()
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
      .chunk_by(|(_, a_name), (_, b_name)| a_name == b_name)
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
    let Net(hvmc::ast::Net { root, redexes }) = self;

    stream.write::<Tree>(&root.clone().into())?;
    stream.write::<VarLenNumber>(&(redexes.len() as u64).into())?;
    stream.write(&redexes.iter().map(|(a, b)| (a.clone().into(), b.clone().into())).collect::<Vec<(Tree, Tree)>>())?;
    stream.write(&self.get_current_wiring())?;

    Ok(())
  }
}

impl<E: Endianness> BitRead<'_, E> for Net {
  fn read(stream: &mut bitbuffer::BitReadStream<'_, E>) -> bitbuffer::Result<Self> {
    let root: Tree = stream.read()?;
    let redexes_len: u64 = stream.read::<VarLenNumber>()?.into();
    let redexes: Vec<(Tree, Tree)> = stream.read_sized(redexes_len as usize)?;

    let mut net: Net = hvmc::ast::Net {
      root: root.into(),
      redexes: redexes.into_iter().map(|(a, b)| (a.into(), b.into())).collect::<Vec<_>>(),
    }
    .into();

    let wiring: Wiring = stream.read_sized(net.gather_vars().len() / 2)?;

    net.apply_wiring(wiring);

    Ok(net)
  }
}

impl Serialize for Net {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_bytes(&encode(self))
  }
}

impl<'de> Deserialize<'de> for Net {
  fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
    Ok(decode(&bytes))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::str::FromStr;

  #[test]
  fn test_net_encoding() {
    let cases = [
      "*",
      "* & * ~ *",
      "(a a)",
      "a & * ~ (b R) & (b a) ~ (R *)",
      "(a (a b)) & (b *) ~ (c c)",
      "((a {a b}) b)", // Y-Combinator
    ];
    for net in cases {
      let net: Net = hvmc::ast::Net::from_str(net).unwrap().into();
      let net_string = format!("{:?}", net);

      let bytes = encode(&net);
      let decoded_net: Net = decode(&bytes);

      assert_eq!(net.normalize(), decoded_net.normalize(), "{}", net_string);
    }
  }
}
