use crate::database::group::arrangement::{Arrangement, ArrangementDetails};
use crate::grouping::arrangement_strategy::ArrangementStrategy;
use crate::grouping::grouping_process::{topological_sort, topological_sort_from};
use crate::grouping::strategy_filtering::{FilterType, StrategyFiltering};
use crate::grouping::strategy_grouping::{StrategyGrouping, TagGrouping};
use std::collections::{HashMap, VecDeque};

#[test]
pub fn test() {
    let mut dequeue = VecDeque::new();
    dequeue.push_back(1);
    dequeue.push_back(2);
    assert_eq!(dequeue.pop_front(), Some(1));
}

pub fn create_arrangement_with_dependant_arrangements(id: u32, dependant_arrangements: Vec<u32>) -> ArrangementDetails {
    ArrangementDetails {
        arrangement: Arrangement {
            id,
            user_id: 0,
            name: "".to_string(),
            strong_match_conversion: false,
            strategy: None,
            groups_dependant: false,
            tags_dependant: false,
            exif_dependant: false,
        },
        strategy: ArrangementStrategy {
            filter: StrategyFiltering {
                filters: vec![vec![FilterType::IncludeGroups(vec![1, 5])]],
            },
            groupings: StrategyGrouping::GroupByTags(TagGrouping {
                tag_group_id: 0,
                tag_id_to_group_id: HashMap::new(),
                other_group_id: None,
                group_names_format: "".to_string(),
            }),
            preserve_unicity: true,
        },
        dependant_groups: vec![],
        dependant_arrangements,
        groups: vec![],
    }
}
pub fn create_arrangement_with_dependant_groups(id: u32, groups: Vec<u32>, dependant_groups: Vec<u32>) -> ArrangementDetails {
    ArrangementDetails {
        arrangement: Arrangement {
            id,
            user_id: 0,
            name: "".to_string(),
            strong_match_conversion: false,
            strategy: None,
            groups_dependant: false,
            tags_dependant: false,
            exif_dependant: false,
        },
        strategy: ArrangementStrategy {
            filter: StrategyFiltering {
                filters: vec![vec![FilterType::IncludeGroups(vec![1, 5])]],
            },
            groupings: StrategyGrouping::GroupByTags(TagGrouping {
                tag_group_id: 0,
                tag_id_to_group_id: HashMap::new(),
                other_group_id: None,
                group_names_format: "".to_string(),
            }),
            preserve_unicity: true,
        },
        dependant_groups,
        dependant_arrangements: vec![],
        groups,
    }
}

#[test]
pub fn test_set_dependants_arrangements_auto() {
    let mut arrangement_1 = create_arrangement_with_dependant_groups(1, vec![10], vec![20, 33]);
    let mut arrangement_2 = create_arrangement_with_dependant_groups(2, vec![20, 21, 22], vec![]);
    let mut arrangement_3 = create_arrangement_with_dependant_groups(3, vec![30, 31, 33], vec![21]);

    let arrangement_details = vec![arrangement_1.clone(), arrangement_2.clone(), arrangement_3.clone()];
    arrangement_1.set_dependant_arrangements_auto(&arrangement_details);
    arrangement_2.set_dependant_arrangements_auto(&arrangement_details);
    arrangement_3.set_dependant_arrangements_auto(&arrangement_details);

    assert_eq!(arrangement_1.dependant_arrangements, vec![2, 3]);
    assert_eq!(arrangement_2.dependant_arrangements, Vec::<u32>::new());
    assert_eq!(arrangement_3.dependant_arrangements, vec![2]);
}

#[test]
pub fn test_topological_sort_1() {
    let arrangements = vec![
        create_arrangement_with_dependant_arrangements(1, vec![3, 4]),
        create_arrangement_with_dependant_arrangements(2, vec![1, 3, 4]),
        create_arrangement_with_dependant_arrangements(3, vec![4]),
        create_arrangement_with_dependant_arrangements(4, vec![]),
        create_arrangement_with_dependant_arrangements(5, vec![]),
    ];

    let mut sorted: Vec<u32> = topological_sort(arrangements).iter().map(|a| a.arrangement.id).collect();
    sorted.retain(|id| id != &5);
    assert_eq!(sorted, vec![4, 3, 1, 2]);
}
#[test]
pub fn test_topological_sort_2() {
    let arrangements = vec![
        create_arrangement_with_dependant_arrangements(1, vec![2, 5]),
        create_arrangement_with_dependant_arrangements(2, vec![]),
        create_arrangement_with_dependant_arrangements(3, vec![2, 4]),
        create_arrangement_with_dependant_arrangements(4, vec![2, 1]),
        create_arrangement_with_dependant_arrangements(5, vec![2]),
    ];

    let sorted: Vec<u32> = topological_sort(arrangements).iter().map(|a| a.arrangement.id).collect();

    assert_eq!(sorted, vec![2, 5, 1, 4, 3]);
}
#[test]
pub fn test_topological_sort_from_1() {
    let arrangements = vec![
        create_arrangement_with_dependant_arrangements(1, vec![2, 5]),
        create_arrangement_with_dependant_arrangements(2, vec![]),
        create_arrangement_with_dependant_arrangements(3, vec![2, 4]),
        create_arrangement_with_dependant_arrangements(4, vec![2, 1]),
        create_arrangement_with_dependant_arrangements(5, vec![2]),
    ];
    let origin = arrangements.iter().find(|a| a.arrangement.id == 1).unwrap().clone();

    let sorted: Vec<u32> = topological_sort_from(arrangements, &origin).iter().map(|a| a.arrangement.id).collect();

    assert_eq!(sorted, vec![1, 4, 3]);
}
#[test]
pub fn test_topological_sort_from_2() {
    let arrangements = vec![
        create_arrangement_with_dependant_arrangements(1, vec![2, 5]),
        create_arrangement_with_dependant_arrangements(2, vec![]),
        create_arrangement_with_dependant_arrangements(3, vec![2, 4]),
        create_arrangement_with_dependant_arrangements(4, vec![2, 1]),
        create_arrangement_with_dependant_arrangements(5, vec![2]),
        create_arrangement_with_dependant_arrangements(6, vec![3, 4]),
    ];
    let origin = arrangements.iter().find(|a| a.arrangement.id == 4).unwrap().clone();

    let sorted: Vec<u32> = topological_sort_from(arrangements, &origin).iter().map(|a| a.arrangement.id).collect();

    assert_eq!(sorted, vec![4, 3, 6]);
}
#[test]
pub fn test_topological_sort_from_3() {
    let arrangements = vec![
        create_arrangement_with_dependant_arrangements(1, vec![5]),
        create_arrangement_with_dependant_arrangements(2, vec![]),
        create_arrangement_with_dependant_arrangements(3, vec![2, 4]),
        create_arrangement_with_dependant_arrangements(4, vec![2, 1]),
        create_arrangement_with_dependant_arrangements(5, vec![2]),
        create_arrangement_with_dependant_arrangements(6, vec![3, 4]),
    ];
    let origin = arrangements.iter().find(|a| a.arrangement.id == 2).unwrap().clone();

    let sorted: Vec<u32> = topological_sort_from(arrangements, &origin).iter().map(|a| a.arrangement.id).collect();

    assert_eq!(sorted, vec![2, 5, 1, 4, 3, 6]);
}
