use crate::database::group::arrangement::{ArrangementDependencyType, ArrangementDetails};
use rocket::{debug, info};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};

/// Sort the arrangements in topological order, keeping only the subtree being the origin arrangement and its dependants.
/// - First explore all arrangements that depend on the origin arrangement
/// - Then apply a topological sort on these arrangements only.
pub fn topological_sort_from(arrangements: Vec<ArrangementDetails>, origin_arrangement: &ArrangementDetails) -> Vec<ArrangementDetails> {
    let mut visited: HashSet<i32> = HashSet::new();
    let mut processing: VecDeque<i32> = VecDeque::new(); // arrangement_id, group_ids

    visited.insert(origin_arrangement.arrangement.id);
    processing.push_back(origin_arrangement.arrangement.id);

    // Process the arrangements that depend on the processing arrangement
    while let Some(processing_id) = processing.pop_front() {
        let new_processing_ids = arrangements
            .iter()
            // Keep only arrangements that are not already visited
            .filter(|a| !visited.contains(&a.arrangement.id))
            // Keep only arrangements that depend on processing_a
            .filter(|a| a.dependant_arrangements.contains(&processing_id))
            .map(|a| a.arrangement.id)
            .collect::<Vec<i32>>();

        for new_processing_id in new_processing_ids {
            visited.insert(new_processing_id);
            processing.push_back(new_processing_id);
        }
    }

    // Remove arrangements that have not been visited
    let arrangements = arrangements
        .into_iter()
        .filter(|a| visited.contains(&a.arrangement.id))
        .collect::<Vec<ArrangementDetails>>();

    // Sort topologically the remaining arrangements.
    topological_sort(arrangements)
}

/// Sort the arrangements in topological order, keeping only the arrangements that match a dependency type, and its dependants.
/// - Gather all arrangements that match the dependency type.
/// - Then add up all arrangements that depend on one of the gathered arrangements.
/// - Finally, apply a topological sort on these arrangements only.
pub fn topological_sort_filtered(arrangements: Vec<ArrangementDetails>, dependency_type: &ArrangementDependencyType) -> Vec<ArrangementDetails> {
    let mut visited: HashSet<i32> = HashSet::new();
    let mut processing: VecDeque<i32> = VecDeque::new(); // arrangement_id, group_ids

    arrangements.iter().filter(|a| dependency_type.match_any(&(*a).into())).for_each(|a| {
        visited.insert(a.arrangement.id);
        processing.push_back(a.arrangement.id);
    });

    // Process the arrangements that depend on the processing arrangement
    while let Some(processing_id) = processing.pop_front() {
        let new_processing_ids = arrangements
            .iter()
            // Keep only arrangements that are not already visited
            .filter(|a| !visited.contains(&a.arrangement.id))
            // Keep only arrangements that depend on processing_a
            .filter(|a| a.dependant_arrangements.contains(&processing_id))
            .map(|a| a.arrangement.id)
            .collect::<Vec<i32>>();

        for new_processing_id in new_processing_ids {
            visited.insert(new_processing_id);
            processing.push_back(new_processing_id);
        }
    }

    // Remove arrangements that have not been visited
    let arrangements = arrangements
        .into_iter()
        .filter(|a| visited.contains(&a.arrangement.id))
        .collect::<Vec<ArrangementDetails>>();

    // Sort topologically the remaining arrangements.
    topological_sort(arrangements)
}

/// Topologically sort the arrangements in function of their dependencies over a group of another arrangement.
/// If A depends on B, B will appear before A in the sorted list.
pub fn topological_sort(mut arrangements: Vec<ArrangementDetails>) -> Vec<ArrangementDetails> {
    let mut sorted = Vec::new();
    let mut visited = HashSet::new();
    let mut temp_stack = HashSet::new();

    let mut id_map: HashMap<i32, &ArrangementDetails> = HashMap::new();
    for arrangement in &arrangements {
        id_map.insert(arrangement.arrangement.id, arrangement);
    }

    debug!(
        "Sorting topologically arrangements: {:?}",
        arrangements.iter().map(|a| a.arrangement.id).collect::<Vec<i32>>()
    );
    // Recursive DFS helper for topological sort
    fn visit<'a>(
        node_id: i32,
        id_map: &'a HashMap<i32, &'a ArrangementDetails>,
        visited: &mut HashSet<i32>,
        temp_stack: &mut HashSet<i32>,
        sorted: &mut Vec<i32>,
    ) -> Result<(), String> {
        // Detect a cycle
        if temp_stack.contains(&node_id) {
            info!("Cycle detected in dependency graph");
            return Ok(()); //Err("Cycle detected in dependency graph".to_string());
        }
        if visited.contains(&node_id) {
            return Ok(()); // Already processed
        }

        // Temporarily mark this node
        temp_stack.insert(node_id);

        debug!("    Looking for dependents of arrangement {}", node_id);
        // Process all dependencies of this node
        if let Some(node) = id_map.get(&node_id) {
            for &dep in &node.dependant_arrangements {
                debug!("      Found dependency of {} : {}", node_id, dep);
                visit(dep, id_map, visited, temp_stack, sorted)?;
            }
        }

        // Mark this node as fully processed and add to the result
        temp_stack.remove(&node_id);
        visited.insert(node_id);
        sorted.push(node_id);
        Ok(())
    }

    // Execute the topological sort for all nodes
    for arrangement in id_map.values() {
        debug!("  Starting DFS from arrangement ID: {}", arrangement.arrangement.id);
        let _res = visit(arrangement.arrangement.id, &id_map, &mut visited, &mut temp_stack, &mut sorted);
    }

    let sorted_indices: HashMap<i32, usize> = sorted.iter().enumerate().map(|(i, &id)| (id, i)).collect();

    // Sort the owned values
    arrangements.sort_by(|a, b| {
        if let Some(i) = sorted_indices.get(&a.arrangement.id) {
            if let Some(i2) = sorted_indices.get(&b.arrangement.id) {
                return i.cmp(i2);
            }
            return Ordering::Less;
        }
        Ordering::Greater
    });
    arrangements
}
