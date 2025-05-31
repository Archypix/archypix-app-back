use crate::database::database::DBConn;
use crate::database::group::arrangement::ArrangementDetails;
use crate::database::group::group::Group;
use crate::database::picture::picture_tag::PictureTag;
use crate::database::tag::tag::Tag;
use crate::grouping::grouping_process::{group_add_pictures, group_remove_pictures};
use crate::grouping::strategy_filtering::{FilterType, StrategyFiltering};
use crate::grouping::strategy_grouping::{StrategyGroupingTrait, UngroupRecord};
use crate::utils::errors_catcher::ErrorResponder;
use itertools::Itertools;
use rocket::serde::{Deserialize, Serialize};
use rocket_okapi::JsonSchema;
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TagGroupingRequest {
    pub tag_group_id: i32,
    pub group_names_format: String,
}
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TagGrouping {
    pub tag_group_id: i32,
    pub tag_id_to_group_id: HashMap<i32, i32>,
    pub other_group_id: Option<i32>,
    pub group_names_format: String,
}
impl TagGrouping {
    fn get_or_create_tag_group(&mut self, conn: &mut DBConn, tag: &Tag, arrangement_id: i32) -> Result<(i32, bool), ErrorResponder> {
        if let Some(id) = self.tag_id_to_group_id.get(&tag.id) {
            Ok((*id, false))
        } else {
            let id = Group::insert(conn, arrangement_id, self.format_group_name(&tag), false)?.id;
            self.other_group_id = Some(id);
            Ok((id, true))
        }
    }
    fn get_or_create_other_group(&mut self, conn: &mut DBConn, arrangement_id: i32) -> Result<(i32, bool), ErrorResponder> {
        if let Some(id) = self.other_group_id {
            Ok((id, false))
        } else {
            let id = Group::insert(conn, arrangement_id, "Other".to_string(), false)?.id;
            self.other_group_id = Some(id);
            Ok((id, true))
        }
    }
    pub fn format_group_name(&self, tag: &Tag) -> String {
        tag.name.clone()
    }
}
impl StrategyGroupingTrait for TagGrouping {
    type Request = TagGroupingRequest;

    fn get_groups(&self) -> Vec<i32> {
        let mut groups: Vec<i32> = self.tag_id_to_group_id.values().cloned().collect();
        if let Some(id) = self.other_group_id {
            (&mut groups).push(id);
        }
        groups
    }

    fn group_pictures(
        &mut self,
        conn: &mut DBConn,
        arrangement_id: i32,
        preserve_unicity: bool,
        ungroup_record: &mut UngroupRecord,
        picture_ids: &HashSet<i64>,
    ) -> Result<bool, ErrorResponder> {
        let mut update_strategy = false;
        let mut remaining_pictures_ids = picture_ids.clone();

        let tags = Tag::list_tags(conn, self.tag_group_id)?;
        for tag in tags {
            let pictures_to_group = if preserve_unicity { &remaining_pictures_ids } else { &picture_ids };

            let group_pictures: HashSet<i64> =
                HashSet::from_iter(PictureTag::filter_pictures_from_tag(conn, tag.id, &pictures_to_group.iter().cloned().collect_vec())?.into_iter());
            remaining_pictures_ids = remaining_pictures_ids.difference(&group_pictures).cloned().collect();

            if group_pictures.len() != 0 {
                let (group_id, group_created) = self.get_or_create_tag_group(conn, &tag, arrangement_id)?;
                update_strategy |= group_created;
                remaining_pictures_ids.retain(|&x| group_pictures.contains(&x));
                group_add_pictures(conn, group_id, &group_pictures.iter().cloned().collect_vec())?;
            }

            if ungroup_record.enable {
                if let Some(group_id) = self.tag_id_to_group_id.get(&tag.id) {
                    let ungroup_pictures = picture_ids.difference(&group_pictures).cloned().collect();
                    ungroup_record.add(*group_id, ungroup_pictures);
                }
            }
        }
        if remaining_pictures_ids.len() != 0 {
            let (other_group_id, group_created) = self.get_or_create_other_group(conn, arrangement_id)?;
            update_strategy = group_created;
            group_add_pictures(conn, other_group_id, &remaining_pictures_ids.iter().cloned().collect_vec())?;
        }
        // If the other group is not just created, and there is an other group, remove the other group pictures.
        if ungroup_record.enable && !update_strategy && self.other_group_id.is_some() {
            let ungroup_pictures = picture_ids.difference(&remaining_pictures_ids).cloned().collect();
            ungroup_record.add(self.other_group_id.unwrap(), ungroup_pictures);
        }
        Ok(update_strategy)
    }

    fn create(conn: &mut DBConn, arrangement_id: i32, request: &Self::Request) -> Result<Box<Self>, ErrorResponder> {
        todo!()
    }

    fn edit(&mut self, conn: &mut DBConn, arrangement_id: i32, request: &Self::Request) -> Result<(), ErrorResponder> {
        todo!()
    }

    fn delete(&self, conn: &mut DBConn, arrangement_id: i32) -> Result<(), ErrorResponder> {
        todo!()
    }
}
