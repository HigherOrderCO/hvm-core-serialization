use bitbuffer::{BitReadSized, BitWrite, Endianness};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Wiring {
  /// How do the VAR ports link to each other, in an ordered list
  pub connections: Vec<(usize, usize)>,
}

impl Wiring {
  pub fn new(mut connections: Vec<(usize, usize)>) -> Self {
    connections.sort_by(|(a_idx, _), (b_idx, _)| a_idx.cmp(b_idx));
    Self { connections }
  }
}

impl<E: Endianness> BitWrite<E> for Wiring {
  fn write(&self, stream: &mut bitbuffer::BitWriteStream<E>) -> bitbuffer::Result<()> {
    debug_assert!(self.connections.iter().is_sorted_by_key(|(a, _)| a));
    let mut ports = vec![false; self.connections.len() * 2];

    for &(this, other_global) in self.connections.iter() {
      ports[this] = true;

      let remaining_ports = ports.iter().enumerate().filter(|(_, b)| !**b).collect::<Vec<_>>();
      let remaining_bits = (remaining_ports.len() as f64).log2().ceil() as usize;
      let other_local = remaining_ports.iter().position(|&(i, _)| i == other_global).unwrap();

      ports[other_global] = true;

      stream.write_int(other_local, remaining_bits)?;
    }
    debug_assert_eq!(ports, vec![true; ports.len()]);
    Ok(())
  }
}

impl<E: Endianness> BitReadSized<'_, E> for Wiring {
  fn read(stream: &mut bitbuffer::BitReadStream<'_, E>, size: usize) -> bitbuffer::Result<Self> {
    let mut ports = vec![false; size * 2];
    let mut connections = Vec::with_capacity(size);

    for _ in 0..size {
      // First unfilled port
      let this = ports.iter().position(|&b| !b).unwrap();
      ports[this] = true;

      // When there is only one remaining port, the remaining_bits will be 0
      // and the other_local will be 0, which is the only remaining port
      let remaining_ports = ports.iter()
        .enumerate() // (global index, is filled)
        .filter(|&(_gi, &b)| !b)
        .enumerate() // (local index, (global index, is filled))
        .collect::<Vec<_>>();
      let remaining_bits = (remaining_ports.len() as f64).log2().ceil() as usize;
      let other_local = stream.read_int(remaining_bits)?;
      let (_, (other_global, _)) = remaining_ports.into_iter().find(|&(li, (_gi, _b))| li == other_local).unwrap();
      ports[other_global] = true;

      connections.push((this, other_global));
    }
    debug_assert_eq!(ports, vec![true; size * 2], "Wiring not fully connected");
    Ok(Wiring { connections })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{decode_sized, encode};

  #[test]
  fn test_wiring_encoding_single() {
    let wiring = Wiring::new(vec![(0, 1)]);

    let bytes = encode(&wiring);
    let decoded_wiring: Wiring = decode_sized(&bytes, wiring.connections.len());
    assert_eq!(wiring, decoded_wiring);
  }

  #[test]
  fn test_wiring_encoding_complex() {
    let wiring = Wiring::new(vec![(0, 10), (1, 11), (2, 5), (3, 4), (6, 7), (8, 9)]);

    let bytes = encode(&wiring);
    let decoded_wiring: Wiring = decode_sized(&bytes, wiring.connections.len());
    assert_eq!(wiring, decoded_wiring);
  }
}
