use indexmap::IndexSet;
use rgb::RGBA8;

use crate::{
    colors::{BitDepth, ColorType},
    headers::IhdrData,
    png::{PngImage, scan_lines::ScanLine},
};

/// Attempt to reduce the number of colors in the palette, returning the reduced image if successful
#[must_use]
pub fn reduced_palette(png: &PngImage, optimize_alpha: bool) -> Option<PngImage> {
    if png.ihdr.bit_depth != BitDepth::Eight {
        return None;
    }
    let ColorType::Indexed { palette } = &png.ihdr.color_type else {
        return None;
    };

    let mut used = [false; 256];
    for &byte in &png.data {
        used[byte as usize] = true;
    }

    let black = RGBA8::new(0, 0, 0, 255);
    let mut condensed = IndexSet::with_capacity(palette.len());
    let mut byte_map = [0; 256];
    let mut did_change = false;
    for (i, used) in used.iter().enumerate() {
        if !used {
            continue;
        }
        // There are invalid files that use pixel indices beyond palette size
        let color = *palette.get(i).unwrap_or(&black);
        byte_map[i] = add_color_to_set(color, &mut condensed, optimize_alpha);
        if byte_map[i] as usize != i {
            did_change = true;
        }
    }

    let data = if did_change {
        // Reassign data bytes to new indices
        png.data.iter().map(|b| byte_map[*b as usize]).collect()
    } else if condensed.len() != palette.len() {
        // Data is unchanged but palette is different size
        // Note the new palette could potentially be larger if the original had a missing entry
        png.data.clone()
    } else {
        // Nothing has changed
        return None;
    };

    let palette: Vec<_> = condensed.into_iter().collect();

    Some(PngImage {
        ihdr: IhdrData {
            color_type: ColorType::Indexed { palette },
            ..png.ihdr
        },
        data,
    })
}

fn add_color_to_set(mut color: RGBA8, set: &mut IndexSet<RGBA8>, optimize_alpha: bool) -> u8 {
    // If there are multiple fully transparent entries, reduce them into one
    if optimize_alpha && color.a == 0 {
        color.r = 0;
        color.g = 0;
        color.b = 0;
    }
    let (idx, _) = set.insert_full(color);
    idx as u8
}

/// Attempt to sort the colors in the palette by luma, returning the sorted image if successful
#[must_use]
pub fn sorted_palette(png: &PngImage) -> Option<PngImage> {
    if png.ihdr.bit_depth != BitDepth::Eight {
        return None;
    }
    let palette = match &png.ihdr.color_type {
        ColorType::Indexed { palette } if palette.len() > 1 => palette,
        _ => return None,
    };

    let mut enumerated: Vec<_> = palette.iter().enumerate().collect();
    // Put the most popular edge color first, which can help slightly if the filter bytes are 0
    let keep_first = most_popular_edge_color(palette.len(), png);
    let first = keep_first.map(|f| enumerated.remove(f));

    // Sort the palette
    enumerated.sort_by(|a, b| {
        // Sort by ascending alpha and descending luma
        let color_val = |color: &RGBA8| {
            let a = i32::from(color.a);
            // Put 7 high bits of alpha first, then luma, then low bit of alpha
            // This provides notable improvement in images with a lot of alpha
            ((a & 0xFE) << 18) + (a & 0x01)
            // These are coefficients for standard sRGB to luma conversion
            - i32::from(color.r) * 299
            - i32::from(color.g) * 587
            - i32::from(color.b) * 114
        };
        color_val(a.1).cmp(&color_val(b.1))
    });
    if let Some(first) = first {
        enumerated.insert(0, first);
    }

    // Extract the new palette and determine if anything changed
    let (remapping, palette): (Vec<_>, Vec<RGBA8>) = enumerated.into_iter().unzip();
    if remapping.iter().enumerate().all(|(a, b)| a == *b) {
        return None;
    }

    // Construct the new mapping and convert the data
    let mut byte_map = [0; 256];
    for (i, &v) in remapping.iter().enumerate() {
        byte_map[v] = i as u8;
    }
    let data = png.data.iter().map(|&b| byte_map[b as usize]).collect();

    Some(PngImage {
        ihdr: IhdrData {
            color_type: ColorType::Indexed { palette },
            ..png.ihdr
        },
        data,
    })
}

/// Sort the colors in the palette using the mzeng technique, returning the sorted image if successful
// (Note: This is currently unused as it is outclassed by the ezeng method)
#[must_use]
pub fn sorted_palette_mzeng(png: &PngImage, matrix: &CoOccurrenceMatrix) -> Option<PngImage> {
    let mut remapping = mzeng_reindex(matrix);
    apply_most_popular_color(png, &mut remapping, matrix);
    apply_palette_reorder(png, &remapping)
}

/// Sort the colors in the palette using the ezeng technique, returning the sorted image if successful
#[must_use]
pub fn sorted_palette_ezeng(
    png: &PngImage,
    matrix: &CoOccurrenceMatrix,
    max_swap_dist: u8,
) -> Option<PngImage> {
    let mut remapping = ezeng_reindex(matrix);
    // Perform additional optimization with pairwise swaps
    pairwise_swap_search(&mut remapping, matrix, max_swap_dist);
    apply_most_popular_color(png, &mut remapping, matrix);
    apply_palette_reorder(png, &remapping)
}

/// Sort the colors in the palette using the battiato technique, returning the sorted image if successful
// (Note: This is currently unused as it is outclassed by the ezeng method)
#[must_use]
pub fn sorted_palette_battiato(png: &PngImage, matrix: &CoOccurrenceMatrix) -> Option<PngImage> {
    let mut remapping = battiato_reindex(matrix);
    apply_most_popular_color(png, &mut remapping, matrix);
    apply_palette_reorder(png, &remapping)
}

// Apply the palette reordering to the image data
fn apply_palette_reorder(png: &PngImage, remapping: &[usize]) -> Option<PngImage> {
    let ColorType::Indexed { palette } = &png.ihdr.color_type else {
        return None;
    };
    assert!(remapping.len() == palette.len());

    // Check if anything changed
    if remapping.iter().enumerate().all(|(a, b)| a == *b) {
        return None;
    }

    // Construct the palette and byte maps and convert the data
    let mut new_palette = Vec::new();
    let mut byte_map = [0; 256];
    for (i, &v) in remapping.iter().enumerate() {
        new_palette.push(palette[v]);
        byte_map[v] = i as u8;
    }
    let data = png.data.iter().map(|&b| byte_map[b as usize]).collect();

    Some(PngImage {
        ihdr: IhdrData {
            color_type: ColorType::Indexed {
                palette: new_palette,
            },
            ..png.ihdr
        },
        data,
    })
}

// Find the most popular color on the image edges (the pixels neighboring the filter bytes)
fn most_popular_edge_color(num_colors: usize, png: &PngImage) -> Option<usize> {
    let mut counts = [0_u32; 256];
    for line in png.scan_lines(false) {
        if let &[first, .., last] = line.data {
            counts[first as usize] += 1;
            counts[last as usize] += 1;
        }
    }
    let max = counts
        .iter()
        .take(num_colors)
        .enumerate()
        .max_by_key(|&(_, v)| v)
        .unwrap();
    // Ensure there's a clear winner - return None if multiple colors are tied
    let max_equal = counts.iter().filter(|&v| v == max.1).count();
    if max_equal > 1 {
        return None;
    }
    Some(max.0)
}

// Put the most popular color first
fn apply_most_popular_color(png: &PngImage, remapping: &mut [usize], matrix: &CoOccurrenceMatrix) {
    let most_popular = matrix.most_popular_color();
    // If the most popular color is less than 15% of the image, don't use it
    if most_popular.1 < png.data.len() as u32 * 3 / 20 {
        return;
    }
    let first_idx = remapping.iter().position(|&i| i == most_popular.0).unwrap();
    // If the index is past halfway, reverse the order so as to minimize the change
    if first_idx >= remapping.len() / 2 {
        remapping.reverse();
        remapping.rotate_right(first_idx + 1);
    } else {
        remapping.rotate_left(first_idx);
    }
}

// Apply a greedy index assignment using the modified version of Zeng's techinque from
// "A note on Zeng's technique for color reindexing of palette-based images" by Pinho et al
// https://ieeexplore.ieee.org/document/1261987
// Based on the C implementation in libwebp
fn mzeng_reindex(matrix: &CoOccurrenceMatrix) -> Vec<usize> {
    // Initialize the mapping list with the two best indices.
    let edges = &matrix.weighted_edges;
    let mut remapping = vec![edges[0].0 as usize, edges[0].1 as usize];

    // Initialize the sums with the first two remappings and find the best one
    let mut sums = Vec::new();
    let mut best_sum_pos = 0;
    let mut best_sum = (0, 0);
    for i in 0..matrix.num_colors {
        let m_row = matrix.row(i);
        if i == remapping[0] || i == remapping[1] {
            continue;
        }
        let sum = (i, m_row[remapping[0]] + m_row[remapping[1]]);
        if sum.1 > best_sum.1 {
            best_sum_pos = sums.len();
            best_sum = sum;
        }
        sums.push(sum);
    }

    while !sums.is_empty() {
        let best_index = best_sum.0;
        // Compute delta to know if we need to prepend or append the best index.
        let mut delta: isize = 0;
        let n = (matrix.num_colors - sums.len()) as isize;
        let best_row = matrix.row(best_index);
        for (i, &index) in remapping.iter().enumerate() {
            delta += (n - 1 - 2 * i as isize) * best_row[index] as isize;
        }
        if delta > 0 {
            remapping.insert(0, best_index);
        } else {
            remapping.push(best_index);
        }
        // Remove best_sum from sums.
        sums.swap_remove(best_sum_pos);
        if !sums.is_empty() {
            // Update all the sums and find the best one.
            best_sum_pos = 0;
            best_sum = (0, 0);
            for (i, sum) in sums.iter_mut().enumerate() {
                sum.1 += best_row[sum.0];
                if sum.1 > best_sum.1 {
                    best_sum_pos = i;
                    best_sum = *sum;
                }
            }
        }
    }

    // Return the completed remapping
    remapping
}

// Apply a different version of Zeng's technique where the best index is inserted at a position
// minimizing total cost increase, rather than just the ends. This version is significantly more
// effective, but is more computationally expensive.
// The "e" here could mean enhanced, extended, exhaustive, or perhaps elephant if you prefer.
fn ezeng_reindex(matrix: &CoOccurrenceMatrix) -> Vec<usize> {
    // Initialize the mapping list with the two best indices.
    let edges = &matrix.weighted_edges;
    let mut remapping = vec![edges[0].0 as usize, edges[0].1 as usize];

    // Initialize the sums with the first two remappings and find the best one
    let mut sums = Vec::new();
    let mut best_sum_pos = 0;
    let mut best_sum = (0, 0);
    for i in 0..matrix.num_colors {
        let m_row = matrix.row(i);
        if i == remapping[0] || i == remapping[1] {
            continue;
        }
        let sum = (i, m_row[remapping[0]] + m_row[remapping[1]]);
        if sum.1 > best_sum.1 {
            best_sum_pos = sums.len();
            best_sum = sum;
        }
        sums.push(sum);
    }

    while !sums.is_empty() {
        let best_index = best_sum.0;
        // Try all insertion positions and pick the one minimizing total cost increase.
        // The cost increase of inserting at position p has two components:
        // 1. New element cost: Σ w(new, placed[k]) * distance(p, k_after_shift)
        // 2. Cross-pair cost: Σ w(placed[a], placed[b]) for all pairs straddling p
        //    (these pairs get pushed 1 unit farther apart by the insertion)
        let m = remapping.len();
        let mut best_pos = 0;
        let mut best_cost = i64::MAX;
        let mut cross_cost: i64 = 0;
        let best_row = matrix.row(best_index);
        for p in 0..=m {
            // New element's weighted distance to all existing elements
            let new_cost: i64 = (0..m)
                .map(|k| {
                    let dist = if k < p { p - k } else { k + 1 - p };
                    best_row[remapping[k]] as i64 * dist as i64
                })
                .sum();

            let total = new_cost + cross_cost;
            if total < best_cost {
                best_cost = total;
                best_pos = p;
            }

            // Update cross_cost for position p+1:
            // Element at position p moves from "right of split" to "left of split"
            // cross_cost(p+1) - cross_cost(p) = Σ_{b>p} w(p,b) - Σ_{a<p} w(a,p)
            if p < m {
                let row_p = matrix.row(remapping[p]);
                for &rb in &remapping[(p + 1)..m] {
                    cross_cost += row_p[rb] as i64;
                }
                for &ra in &remapping[..p] {
                    cross_cost -= row_p[ra] as i64;
                }
            }
        }
        remapping.insert(best_pos, best_index);

        // Remove best_sum from sums.
        sums.swap_remove(best_sum_pos);
        if !sums.is_empty() {
            // Update all the sums and find the best one.
            best_sum_pos = 0;
            best_sum = (0, 0);
            for (i, sum) in sums.iter_mut().enumerate() {
                sum.1 += best_row[sum.0];
                if sum.1 > best_sum.1 {
                    best_sum_pos = i;
                    best_sum = *sum;
                }
            }
        }
    }

    // Return the completed remapping
    remapping
}

// Calculate an approximate solution of the Traveling Salesman Problem using the algorithm
// from "An efficient Re-indexing algorithm for color-mapped images" by Battiato et al
// https://ieeexplore.ieee.org/document/1344033
fn battiato_reindex(matrix: &CoOccurrenceMatrix) -> Vec<usize> {
    let mut chains = Vec::new();
    // Keep track of the state of each vertex (.0) and it's chain number (.1)
    // 0 = an unvisited vertex (White)
    // 1 = an endpoint of a chain (Red)
    // 2 = part of the middle of a chain (Black)
    let mut vx = vec![(0, 0); matrix.num_colors];

    // Iterate the edges and assemble them into a chain
    for &(i, j, _) in &matrix.weighted_edges {
        let i = i as usize;
        let j = j as usize;
        let vi = vx[i];
        let vj = vx[j];
        if vi.0 == 0 && vj.0 == 0 {
            // Two unvisited vertices - create a new chain
            vx[i].0 = 1;
            vx[i].1 = chains.len();
            vx[j].0 = 1;
            vx[j].1 = chains.len();
            chains.push(vec![i, j]);
        } else if vi.0 == 0 && vj.0 == 1 {
            // An unvisited vertex connects with an endpoint of an existing chain
            vx[i].0 = 1;
            vx[i].1 = vj.1;
            vx[j].0 = 2;
            let chain = &mut chains[vj.1];
            if chain[0] == j {
                chain.insert(0, i);
            } else {
                chain.push(i);
            }
        } else if vi.0 == 1 && vj.0 == 0 {
            // An unvisited vertex connects with an endpoint of an existing chain
            vx[j].0 = 1;
            vx[j].1 = vi.1;
            vx[i].0 = 2;
            let chain = &mut chains[vi.1];
            if chain[0] == i {
                chain.insert(0, j);
            } else {
                chain.push(j);
            }
        } else if vi.0 == 1 && vj.0 == 1 && vi.1 != vj.1 {
            // Two endpoints of different chains are connected together
            vx[i].0 = 2;
            vx[j].0 = 2;
            let (a, b) = if vi.1 < vj.1 { (i, j) } else { (j, i) };
            let ca = vx[a].1;
            let cb = vx[b].1;
            let chainb = std::mem::take(&mut chains[cb]);
            for &v in &chainb {
                vx[v].1 = ca;
            }
            let chaina = &mut chains[ca];
            if chaina[0] == a && chainb[0] == b {
                for v in chainb {
                    chaina.insert(0, v);
                }
            } else if chaina[0] == a {
                chaina.splice(0..0, chainb);
            } else if chainb[0] == b {
                chaina.extend(chainb);
            } else {
                let pos = chaina.len();
                for v in chainb {
                    chaina.insert(pos, v);
                }
            }
        }

        if chains[0].len() == matrix.num_colors {
            break;
        }
    }

    // Since zero-weight edges are skipped we may not have a complete chain yet.
    // Join all remaining chains and add any unvisited vertices to complete the remapping.
    chains
        .into_iter()
        .flatten()
        .chain(
            vx.into_iter()
                .enumerate()
                .filter_map(|(i, v)| if v.0 == 0 { Some(i) } else { None }),
        )
        .collect()
}

// Pairwise swap: for each pair (a, b), swap if it reduces cost.
// This is an effective means of refining the result of another algorithm. It's currently rather
// brutish and not very efficient - there are probably ways it could be optimized, or perhaps the
// ezeng algorithm could be modified to achieve a similar effect without needing this step at all.
//
// `max_dist` limits the distance between pairs to consider to keep it performant. A value of 1 is
// quite fast and still provides a good improvement, but higher values can be a little better.
fn pairwise_swap_search(remapping: &mut [usize], matrix: &CoOccurrenceMatrix, max_dist: u8) {
    let num_colors = remapping.len();
    let b_limit = max_dist as usize + 1;

    // Keep iterating as long as at least two swaps were made
    // When we're down to only one then the chance of any further improvement is practically nil
    let mut swaps = 2;
    while swaps >= 2 {
        swaps = 0;
        for a in 0..num_colors - 1 {
            for b in (a + 1)..(a + b_limit).min(num_colors) {
                let va = remapping[a];
                let vb = remapping[b];
                let row_a = matrix.row(va);
                let row_b = matrix.row(vb);
                let mut delta: i64 = 0;
                // Dist diff is calculated as: (b - i).abs() - (a - i).abs()
                // Split by index ranges so we can simplify the calculation and avoid checks to skip a and b
                let dist = (b - a) as i64;
                for &vi in &remapping[..a] {
                    let weight_diff = row_a[vi] as i64 - row_b[vi] as i64;
                    delta += weight_diff * dist;
                }
                for (off, &vi) in remapping[(a + 1)..b].iter().enumerate() {
                    let i = (a + 1 + off) as i64;
                    let weight_diff = row_a[vi] as i64 - row_b[vi] as i64;
                    let dist_diff = (a + b) as i64 - 2 * i;
                    delta += weight_diff * dist_diff;
                }
                for &vi in &remapping[(b + 1)..] {
                    let weight_diff = row_a[vi] as i64 - row_b[vi] as i64;
                    delta -= weight_diff * dist;
                }
                if delta < 0 {
                    remapping.swap(a, b);
                    swaps += 1;
                }
            }
        }
    }
}

/// The co-occurrence matrix records the number of times each color is adjacent to every other color.
#[derive(Debug)]
pub struct CoOccurrenceMatrix {
    num_colors: usize,
    data: Vec<u32>,
    weighted_edges: Vec<(u8, u8, u32)>,
}
impl CoOccurrenceMatrix {
    /// Construct a co-occurrence matrix from the given image, or return None if it is not supported.
    ///
    /// Pre-condition: The palette must match the image data, with no unused or missing entries.
    /// I.e., the image must have been processed with `reduced_palette` first.
    pub fn from(png: &PngImage) -> Option<Self> {
        if png.ihdr.bit_depth != BitDepth::Eight {
            return None;
        }
        let num_colors = match &png.ihdr.color_type {
            // Images with only two colors will remain unchanged from previous luma sort
            ColorType::Indexed { palette } if palette.len() > 2 => palette.len(),
            _ => return None,
        };
        let data = Self::build(num_colors, png);
        let weighted_edges = Self::weighted_edges(num_colors, &data);
        Some(Self {
            num_colors,
            data,
            weighted_edges,
        })
    }

    /// Construct the co-occurrence data
    fn build(num_colors: usize, png: &PngImage) -> Vec<u32> {
        // A flat array seems to perform better than a 2D array for ezeng and swaps
        let mut data = vec![0; num_colors * num_colors];
        let mut prev: Option<ScanLine> = None;
        for line in png.scan_lines(false) {
            let mut prev_val = None;
            for i in 0..line.data.len() {
                let val = line.data[i] as usize;
                if let Some(prev_val) = prev_val.replace(val) {
                    data[prev_val * num_colors + val] += 1;
                }
                // Use safe access of the previous line bytes in case of interlacing where the line may be shorter.
                // Note: Since filtering doesn't apply across interlacing passes it could be argued that we shouldn't
                // use the previous line on a new pass. However, doing so could theoretically allow colors to be
                // "isolated" and if no edges are formed then zeng doesn't have a starting point. It's easiest to just
                // always use the previous line to ensure no possibility of isolation can occur.
                if let Some(&prev_val) = prev.as_ref().and_then(|l| l.data.get(i)) {
                    data[prev_val as usize * num_colors + val] += 1;
                }
            }
            prev = Some(line);
        }

        // Make the matrix symmetrical - this is faster to do afterward than maintaining symmetry during counting
        for i in 0..num_colors {
            let row_start = i * num_colors;
            for j in 0..=i {
                let opposite = j * num_colors + i;
                data[row_start + j] += data[opposite];
                data[opposite] = data[row_start + j];
            }
        }
        data
    }

    /// Calculate edge list sorted by weight
    fn weighted_edges(num_colors: usize, data: &[u32]) -> Vec<(u8, u8, u32)> {
        let mut edges = Vec::new();
        for i in 0..num_colors {
            let row = &data[(i * num_colors)..];
            for (j, &val) in row.iter().enumerate().take(i) {
                // For performance, skip zero-weight edges since they aren't really edges
                if val > 0 {
                    edges.push((j as u8, i as u8, val));
                }
            }
        }
        edges.sort_by(|(_, _, w1), (_, _, w2)| w2.cmp(w1));
        edges
    }

    /// Get a row of the matrix
    #[inline]
    fn row(&self, row: usize) -> &[u32] {
        let start = row * self.num_colors;
        &self.data[start..(start + self.num_colors)]
    }

    /// Find the most popular color in the matrix, along with its count
    fn most_popular_color(&self) -> (usize, u32) {
        let mut best = (0, 0);
        for i in 0..self.num_colors {
            let sum: u32 = self.row(i).iter().sum();
            if sum > best.1 {
                best = (i, sum);
            }
        }
        // Each color is counted 4 times in the matrix, so divide to get the actual count
        // This is not 100% accurate for colors on the edge of the image, but it's close enough for our purposes
        best.1 /= 4;
        best
    }
}
