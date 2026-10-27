//! Bound expanded interactive YAML before the engine constructs an owned tree.
//! The existing parser retains source events, while this visitor retains only
//! counters. Aliases traverse the same visitor and pay the same expanded cost.

use serde::de::{DeserializeSeed, EnumAccess, Error, MapAccess, SeqAccess, VariantAccess, Visitor};
use std::fmt;

pub(crate) fn check(document: &str) -> Result<(), serde_yaml::Error> {
    Count {
        budget: &mut Budget {
            strings: 1_048_576,
            values: 131_072,
        },
        depth: 1,
    }
    .deserialize(serde_yaml::Deserializer::from_str(document))
}

struct Budget {
    strings: usize,
    values: usize,
}
struct Count<'a> {
    budget: &'a mut Budget,
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for Count<'_> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        if self.depth > 64 {
            return Err(D::Error::custom("interactive YAML nesting limit"));
        }
        self.budget.values = self
            .budget
            .values
            .checked_sub(1)
            .ok_or_else(|| D::Error::custom("interactive YAML value limit"))?;
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Count<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("bounded interactive YAML")
    }
    fn visit_bool<E: Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E: Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_str<E: Error>(self, value: &str) -> Result<(), E> {
        self.budget.strings = self
            .budget
            .strings
            .checked_sub(value.len())
            .ok_or_else(|| E::custom("interactive YAML string limit"))?;
        Ok(())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<(), A::Error> {
        while sequence
            .next_element_seed(Count {
                budget: &mut *self.budget,
                depth: self.depth + 1,
            })?
            .is_some()
        {}
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut mapping: A) -> Result<(), A::Error> {
        while mapping
            .next_key_seed(Count {
                budget: &mut *self.budget,
                depth: self.depth + 1,
            })?
            .is_some()
        {
            mapping.next_value_seed(Count {
                budget: &mut *self.budget,
                depth: self.depth + 1,
            })?;
        }
        Ok(())
    }
    fn visit_enum<A: EnumAccess<'de>>(self, tagged: A) -> Result<(), A::Error> {
        let (_, contents) = tagged.variant_seed(Count {
            budget: &mut *self.budget,
            depth: self.depth + 1,
        })?;
        contents.newtype_variant_seed(Count {
            budget: &mut *self.budget,
            depth: self.depth + 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::check;

    #[test]
    fn packaged_and_larger_supported_documents_fit_the_parse_budgets() {
        let kit = include_str!("../../../examples/patches/advanced-drum-kit.yaml");
        assert!(check(kit).is_ok());
        let regions = (0..256)
            .map(|index| {
                format!(
                    "        - {{ id: parse_{index}, start_frame: {}, end_frame: {} }}\n",
                    index * 32,
                    index * 32 + 16
                )
            })
            .collect::<String>();
        let larger = kit.replacen("      regions:\n", &format!("      regions:\n{regions}"), 1);
        assert!(larger.len() > 16_384 && larger.len() < 65_536);
        dandrum_engine::kernel::document::load_kernel_patch_str(&larger).unwrap();
        assert!(check(&larger).is_ok());
    }

    #[test]
    fn ordinary_scalars_collections_aliases_and_tags_share_the_counted_parser() {
        for document in [
            "{value: [true, -1, 1, 0.25, null, '& *', \"quoted\\ntext\"]}",
            "[&row {value: 2}, *row, *row]",
            "!voice [true, -1, 1, 0.25, null, '& *']",
            "{!label key: !gain 0.5}",
        ] {
            assert!(check(document).is_ok(), "{document}");
        }
        for invalid in [
            "[",
            "[1,",
            "a: *missing",
            "---\n1\n---\n2",
            "18446744073709551616",
            "{18446744073709551616: 0}",
        ] {
            assert!(check(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn expanded_string_budget_counts_utf8_bytes_at_the_inclusive_limit() {
        for text in ["x".repeat(16_384), "é".repeat(8192)] {
            let exact = format!(
                "[&text '{text}',{}]",
                std::iter::repeat_n("*text", 63)
                    .collect::<Vec<_>>()
                    .join(",")
            );
            assert!(exact.len() < 65_536);
            assert!(check(&exact).is_ok());
            let over = format!("{},'é']", exact.strip_suffix(']').unwrap());
            assert!(check(&over).is_err());
        }
    }

    #[test]
    fn expanded_value_and_nesting_budgets_reject_the_first_excess() {
        // One outer list, 127 lists of 1,023 numbers, and 1,023 additional
        // numbers: 1 + 127 * 1,024 + 1,023 = 131,072 expanded values.
        let numbers = std::iter::repeat_n("0", 1023).collect::<Vec<_>>().join(",");
        let aliases = std::iter::repeat_n("*row", 126)
            .collect::<Vec<_>>()
            .join(",");
        let exact = format!("[&row [{numbers}],{aliases},{numbers}]");
        assert!(exact.len() < 65_536);
        assert!(check(&exact).is_ok());
        assert!(check(&format!("{},0]", exact.strip_suffix(']').unwrap())).is_err());
        let nested =
            |levels: usize| format!("{}0{}", "[".repeat(levels - 1), "]".repeat(levels - 1));
        assert!(check(&nested(64)).is_ok());
        assert!(check(&nested(65)).is_err());
        let deep_tag = format!("{}!tag 0{}", "[".repeat(63), "]".repeat(63));
        assert!(check(&deep_tag).is_err());
    }
}
