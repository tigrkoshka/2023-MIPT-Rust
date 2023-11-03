#![forbid(unsafe_code)]

fn combinations_recurse(
    elems: &[i32],
    curr_fill_pos: usize,
    curr_comb: &mut Vec<i32>,
) -> Vec<Vec<i32>> {
    if curr_fill_pos >= curr_comb.len() {
        return vec![curr_comb.clone()];
    }

    let n_possible_elements = elems.len() - (curr_comb.len() - curr_fill_pos) + 1;

    elems
        .iter()
        .take(n_possible_elements)
        .enumerate()
        .flat_map(|(i, elem)| -> Vec<Vec<i32>> {
            curr_comb[curr_fill_pos] = *elem;
            combinations_recurse(&elems[i + 1..], curr_fill_pos + 1, curr_comb)
        })
        .collect()
}

pub fn combinations(arr: &[i32], k: usize) -> Vec<Vec<i32>> {
    if k > arr.len() {
        return vec![]; // no combinations
    }

    let mut curr_comb = vec![0; k];
    combinations_recurse(arr, 0, &mut curr_comb)
}
