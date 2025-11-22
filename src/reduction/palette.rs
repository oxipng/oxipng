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
#[must_use]
pub fn sorted_palette_mzeng(png: &PngImage) -> Option<PngImage> {
    // Interlacing not currently supported
    if png.ihdr.bit_depth != BitDepth::Eight || png.ihdr.interlaced {
        return None;
    }
    let palette = match &png.ihdr.color_type {
        // Images with only two colors will remain unchanged from previous luma sort
        ColorType::Indexed { palette } if palette.len() > 2 => palette,
        _ => return None,
    };

    let matrix = co_occurrence_matrix(palette.len(), png);
    let edges = weighted_edges(&matrix);
    let mut remapping = mzeng_reindex(palette.len(), edges, &matrix);

    apply_most_popular_color(png, &mut remapping);

    apply_palette_reorder(png, &remapping)
}

/// Sort the colors in the palette using the battiato technique, returning the sorted image if successful
#[must_use]
pub fn sorted_palette_battiato(png: &PngImage) -> Option<PngImage> {
    // Interlacing not currently supported
    if png.ihdr.bit_depth != BitDepth::Eight || png.ihdr.interlaced {
        return None;
    }
    let palette = match &png.ihdr.color_type {
        // Images with only two colors will remain unchanged from previous luma sort
        ColorType::Indexed { palette } if palette.len() > 2 => palette,
        _ => return None,
    };

    let matrix = co_occurrence_matrix(palette.len(), png);
    let edges = weighted_edges(&matrix);
    let mut remapping = battiato_reindex(palette.len(), edges);

    apply_most_popular_color(png, &mut remapping);

    apply_palette_reorder(png, &remapping)
}

// Apply the palette reordering to the image data
fn apply_palette_reorder(png: &PngImage, remapping: &[usize]) -> Option<PngImage> {
    let ColorType::Indexed { palette } = &png.ihdr.color_type else {
        return None;
    };

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

// Find the most popular color in the image, along with its count
fn most_popular_color(num_colors: usize, png: &PngImage) -> (usize, u32) {
    let mut counts = [0_u32; 256];
    for &val in &png.data {
        counts[val as usize] += 1;
    }
    counts
        .iter()
        .copied()
        .take(num_colors)
        .enumerate()
        .max_by_key(|&(_, v)| v)
        .unwrap_or_default()
}

// Put the most popular color first
fn apply_most_popular_color(png: &PngImage, remapping: &mut [usize]) {
    let most_popular = most_popular_color(remapping.len(), png);
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

// Calculate co-occurences matrix
fn co_occurrence_matrix(num_colors: usize, png: &PngImage) -> Vec<Vec<u32>> {
    let mut matrix = vec![vec![0_u32; num_colors]; num_colors];
    let mut prev: Option<ScanLine> = None;
    let mut prev_val = None;
    for line in png.scan_lines(false) {
        for i in 0..line.data.len() {
            let val = line.data[i] as usize;
            if val > num_colors {
                continue;
            }
            if let Some(prev_val) = prev_val.replace(val) {
                matrix[prev_val][val] += 1;
                matrix[val][prev_val] += 1;
            }
            if let Some(prev) = &prev {
                let prev_val = prev.data[i] as usize;
                if prev_val > num_colors {
                    continue;
                }
                matrix[prev_val][val] += 1;
                matrix[val][prev_val] += 1;
            }
        }
        prev = Some(line);
    }
    matrix
}

// Calculate edge list sorted by weight
fn weighted_edges(matrix: &[Vec<u32>]) -> Vec<(usize, usize)> {
    let mut edges = Vec::new();
    for (i, m_row) in matrix.iter().enumerate() {
        for (j, val) in m_row.iter().enumerate().take(i) {
            edges.push(((j, i), val));
        }
    }
    edges.sort_by(|(_, w1), (_, w2)| w2.cmp(w1));
    edges.into_iter().map(|(e, _)| e).collect()
}

// Apply a greedy index assignment using the modified version of Zeng's techinque from
// "A note on Zeng's technique for color reindexing of palette-based images" by Pinho et al
// https://ieeexplore.ieee.org/document/1261987
// Based on the C implementation in libwebp
fn mzeng_reindex(num_colors: usize, edges: Vec<(usize, usize)>, matrix: &[Vec<u32>]) -> Vec<usize> {
    // Initialize the mapping list with the two best indices.
    let mut remapping = vec![edges[0].0, edges[0].1];

    // Initialize the sums with the first two remappings and find the best one
    let mut sums = Vec::new();
    let mut best_sum_pos = 0;
    let mut best_sum = (0, 0);
    for (i, m_row) in matrix.iter().enumerate() {
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
        let n = (num_colors - sums.len()) as isize;
        for (i, &index) in remapping.iter().enumerate() {
            delta += (n - 1 - 2 * i as isize) * matrix[best_index][index] as isize;
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
                sum.1 += matrix[best_index][sum.0];
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
fn battiato_reindex(num_colors: usize, edges: Vec<(usize, usize)>) -> Vec<usize> {
    let mut chains = Vec::new();
    // Keep track of the state of each vertex (.0) and it's chain number (.1)
    // 0 = an unvisited vertex (White)
    // 1 = an endpoint of a chain (Red)
    // 2 = part of the middle of a chain (Black)
    let mut vx = vec![(0, 0); num_colors];

    // Iterate the edges and assemble them into a chain
    for (i, j) in edges {
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

        if chains[0].len() == num_colors {
            break;
        }
    }

    // Return the completed chain
    chains.swap_remove(0)
}
