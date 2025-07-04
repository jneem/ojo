// Copyright 2018-2019 Joe Neeman.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
// See the LICENSE-APACHE or LICENSE-MIT files at the top-level directory
// of this distribution.

// Here, we implement the "patience" algorithm for the longest increasing subsequence. In our
// applications of this algorithm, we always expect the sequence to have unique elements, but in
// any case this function will compute the longest *strictly* increasing subsequence.
//
// Here's a quick overview of the algorithm. We maintain a bunch of "stacks," starting with a
// single stack on the left, and adding extra stacks to the right as necessary. Each stack will be
// (non-strictly) ordered, with largest values on the bottom and smaller values on top. We process
// the input sequence from beginning to end, we put it on the left-most stack possible, keeping in
// mind that each stack needs to stay ordered. If we can't put in on any stack, we start a new
// stack on the right.
//
// Here's a quick example: if the input sequence is 4 6 5 3 7, then in the first step we put 4 in its
// own stack:
//
// ```
// 4
// ```
//
// The 6 cannot go on top of the 4, so it starts a new stack.
//
// ```
// 4 6
// ```
//
// The left-most legal position for the 5 is on top of the 6.
//
// ```
//   5
// 4 6
// ```
//
// The 7 goes in its own stack:
//
// ```
//   5
// 4 6 7
// ```
//
// Finally, the 3 goes on top of the 4:
//
// ```
// 3 5
// 4 6 7
// ```
//
// To reconstruct a longest increasing subsequence (LIS), start with the right-most stack. Take the
// element on top (7, in the example above); this will be the last element of the LIS. To find the
// preceeding (second-last) element of the LIS, we rewind back to the time when the last element
// was placed in its stack, and we look at the top of the stack to its left (which, in the example
// above, was a 5). Then we repeat: at the time the 5 was placed, the top of the stack to its left
// held a 4. That is, 4 5 7 is an LIS of the original sequence.
//
// In order to translate this into a more efficient algorithm, we make two observations.
//
// 1. We don't need to record the entire history of the process; instead, we just keep track of the
//    part of the history that we need: every time we add an element to a stack, record that
//    element and the element at the top of the stack to the left.
// 2. We don't even need to store all of the stacks; it's enough to just store the top element of
//    each stack. Also, note that the sequence of top elements forms an increasing sequence, and so
//    each time we process an element, we can use binary search to figure out where it should go.
//
// There's one other way in which the implementation below differs from the description above:
// rather than manipulate the sequence elements directly, we work with indices pointing to them.
// This avoids the need to assume that `T` is cloneable (and in case `T` is large, it might be
// faster).
pub fn longest_increasing_subsequence<T: Ord>(seq: &[T]) -> Vec<usize> {
    if seq.is_empty() {
        return Vec::new();
    }

    // seq[stack_tops[i]] is the element at the top of stack i.
    let mut stack_tops: Vec<usize> = Vec::new();
    // seq[pointers[i]] is the element that was at the top of the stack to the left at the time
    // that seq[i] was placed in a stack. The `usize::MAX` fillers here should never actually be
    // read, assuming there's no bug in the algorithm.
    let mut pointers: Vec<usize> = vec![usize::MAX; seq.len()];

    // Place the elements in stacks, one by one.
    for (elem_idx, elem) in seq.iter().enumerate() {
        let stack_idx = match stack_tops.binary_search_by(|x| seq[*x].cmp(elem)) {
            Ok(idx) => idx,
            Err(idx) => idx,
        };
        if stack_idx >= stack_tops.len() {
            stack_tops.push(elem_idx);
        } else {
            stack_tops[stack_idx] = elem_idx;
        }

        if stack_idx > 0 {
            pointers[elem_idx] = stack_tops[stack_idx - 1];
        }
    }

    // Reconstruct the LIS from the pointers.
    let mut ret = vec![usize::MAX; stack_tops.len()];
    let mut idx = *stack_tops.last().unwrap(); // This will not panic, since seq is non-empty.
    for i in ret.iter_mut().rev() {
        *i = idx;
        idx = pointers[idx];
    }

    ret
}

#[cfg(test)]
mod tests {
    use {super::longest_increasing_subsequence, proptest::prelude::*};

    macro_rules! lis_test {
        ($name:ident, $seq:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let seq: &[usize] = &$seq[..];
                let expected: &[usize] = &$expected[..];
                let lis = longest_increasing_subsequence(seq);
                assert_eq!(lis.as_slice(), expected);
            }
        };
    }

    lis_test!(lis_test_empty, [], []);
    lis_test!(
        lis_test_ordered,
        [0, 1, 2, 3, 4, 5, 6],
        [0, 1, 2, 3, 4, 5, 6]
    );
    lis_test!(lis_test_reversed, [6, 5, 4, 3, 2, 1, 0], [6]);

    proptest! {
        #[test]
        fn output_is_increasing(ref seq in proptest::collection::vec(1..10000, 1..100)) {
            let lis = longest_increasing_subsequence(seq);
            let lis = lis.into_iter().map(|i| seq[i]).collect::<Vec<_>>();
            assert!(lis.windows(2).all(|pair| pair[0] < pair[1]))
        }
    }
}
