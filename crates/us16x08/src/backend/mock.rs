//! An in-memory [`Backend`] for development and tests.

use std::collections::BTreeMap;

use super::Backend;
use crate::control::{Control, Kind};
use crate::error::{Error, Result};
use crate::meter::METER_COUNT;

/// In-memory stand-in for a real card.
///
/// Seeded from [`Control::ALL`] with each control's default value at every
/// index in its scope, so the full control surface is present without
/// hardware. Each element stores one or more integer slots (booleans as `0`/`1`,
/// the meter block as [`METER_COUNT`] slots).
#[derive(Debug, Clone)]
pub struct MockBackend {
    elems: BTreeMap<(String, u32), Vec<i32>>,
}

impl MockBackend {
    /// Build a mock seeded with every control at its default value.
    #[must_use]
    pub fn new() -> Self {
        let mut elems = BTreeMap::new();
        for &control in Control::ALL {
            let name = control.alsa_name().to_owned();
            let slots: Vec<i32> = match control.kind() {
                Kind::Bool => vec![0],
                Kind::Int { default, .. } | Kind::Enum { default, .. } => vec![default],
                Kind::Meter => vec![0; METER_COUNT],
            };
            for index in 0..control.scope().count() {
                elems.insert((name.clone(), index), slots.clone());
            }
        }
        Self { elems }
    }

    /// Directly set the raw slots of an element, e.g. to simulate the hardware
    /// pushing new meter values. Returns an error if the element is unknown.
    pub fn set_raw(&mut self, name: &str, index: u32, slots: &[i32]) -> Result<()> {
        let slot = self.slots_mut(name, index)?;
        slot.clear();
        slot.extend_from_slice(slots);
        Ok(())
    }

    fn slots(&self, name: &str, index: u32) -> Result<&Vec<i32>> {
        self.elems
            .get(&(name.to_owned(), index))
            .ok_or_else(|| Error::UnknownControl {
                name: name.to_owned(),
                index,
            })
    }

    fn slots_mut(&mut self, name: &str, index: u32) -> Result<&mut Vec<i32>> {
        self.elems
            .get_mut(&(name.to_owned(), index))
            .ok_or_else(|| Error::UnknownControl {
                name: name.to_owned(),
                index,
            })
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for MockBackend {
    fn get_int(&self, name: &str, index: u32) -> Result<i32> {
        self.slots(name, index)?
            .first()
            .copied()
            .ok_or_else(|| Error::UnknownControl {
                name: name.to_owned(),
                index,
            })
    }

    fn set_int(&mut self, name: &str, index: u32, val: i32) -> Result<()> {
        let slots = self.slots_mut(name, index)?;
        match slots.first_mut() {
            Some(first) => {
                *first = val;
                Ok(())
            }
            None => Err(Error::UnknownControl {
                name: name.to_owned(),
                index,
            }),
        }
    }

    fn get_bool(&self, name: &str, index: u32) -> Result<bool> {
        Ok(self.get_int(name, index)? != 0)
    }

    fn set_bool(&mut self, name: &str, index: u32, val: bool) -> Result<()> {
        self.set_int(name, index, i32::from(val))
    }

    fn get_ints(&self, name: &str, out: &mut [i32]) -> Result<usize> {
        let slots = self.slots(name, 0)?;
        let n = slots.len().min(out.len());
        for (dst, src) in out.iter_mut().zip(slots.iter()) {
            *dst = *src;
        }
        Ok(n)
    }

    fn elem_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.elems.keys().map(|(name, _)| name.clone()).collect();
        names.sort_unstable();
        names.dedup();
        names
    }
}
