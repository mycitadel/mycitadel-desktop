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
    fn to_tap_tree(self) -> Option<TapTree<Pk>>;
}

impl<Pk> ToTapTree<Pk> for Vec<(u8, Miniscript<Pk, Tap>)>
where
    Pk: MiniscriptKey,
{
    fn to_tap_tree(self) -> Option<TapTree<Pk>> {
        let (tap_tree, remnant) = self.into_iter().rfold(
            (None, None) as (Option<TapTree<Pk>>, Option<Miniscript<Pk, Tap>>),
            |(tree, prev), (depth, ms)| match (tree, prev) {
                (None, None) if depth % 2 == 1 => (None, Some(ms)),
                (None, None) if depth % 2 == 1 => (Some(TapTree::Leaf(Arc::new(ms))), None),
                (None, Some(ms2)) => (
                    Some(TapTree::Tree(
                        Arc::new(TapTree::Leaf(Arc::new(ms))),
                        Arc::new(TapTree::Leaf(Arc::new(ms2))),
                    )),
                    None,
                ),
                (Some(tree), None) => (
                    Some(TapTree::Tree(
                        Arc::new(TapTree::Leaf(Arc::new(ms))),
                        Arc::new(tree),
                    )),
                    None,
                ),
                _ => unreachable!(),
            },
        );

        tap_tree.or_else(|| remnant.map(|ms| TapTree::Leaf(Arc::new(ms))))
    }
}
