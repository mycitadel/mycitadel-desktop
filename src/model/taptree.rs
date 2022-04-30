// MyCitadel desktop wallet: bitcoin & RGB wallet based on GTK framework.
//
// Written in 2022 by
//     Dr. Maxim Orlovsky <orlovsky@pandoraprime.ch>
//
// Copyright (C) 2022 by Pandora Prime Sarl, Switzerland.
//
// This software is distributed without any warranty. You should have received
// a copy of the AGPL-3.0 License along with this software. If not, see
// <https://www.gnu.org/licenses/agpl-3.0-standalone.html>.

use std::sync::Arc;

use miniscript::descriptor::TapTree;
use miniscript::{Miniscript, MiniscriptKey, Tap};

pub trait ToTapTree<Pk>
where
    Pk: MiniscriptKey,
{
    fn to_tap_tree(self) -> Result<TapTree<Pk>, miniscript::Error>;
}

impl<Pk> ToTapTree<Pk> for Vec<(u8, Miniscript<Pk, Tap>)>
where
    Pk: MiniscriptKey,
{
    fn to_tap_tree(self) -> Result<TapTree<Pk>, miniscript::Error> {
        let ms_err = || {
            miniscript::Error::Unexpected(s!(
                "unable to construct TapTree from the given spending conditions"
            ))
        };

        let (tap_tree, remnant) = self.into_iter().try_rfold(
            (None, None) as (Option<TapTree<Pk>>, Option<Miniscript<Pk, Tap>>),
            |(tree, prev), (depth, ms)| match (tree, prev) {
                (None, None) if depth % 2 == 1 => Ok((None, Some(ms))),
                (None, None) if depth % 2 == 0 => Ok((Some(TapTree::Leaf(Arc::new(ms))), None)),
                (None, Some(ms2)) => Ok((
                    Some(TapTree::Tree(
                        Arc::new(TapTree::Leaf(Arc::new(ms))),
                        Arc::new(TapTree::Leaf(Arc::new(ms2))),
                    )),
                    None,
                )),
                (Some(tree), None) => Ok((
                    Some(TapTree::Tree(
                        Arc::new(TapTree::Leaf(Arc::new(ms))),
                        Arc::new(tree),
                    )),
                    None,
                )),
                _ => Err(ms_err()),
            },
        )?;

        tap_tree
            .or_else(|| remnant.map(|ms| TapTree::Leaf(Arc::new(ms))))
            .ok_or(ms_err())
    }
}
