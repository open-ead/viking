use std::collections::BTreeMap;

use cu::pre::*;
use itertools::Itertools;

use crate::demangle;
use crate::fl;

use crate::cmd_progress::GroupType;

pub struct Builder {
    report: Report,
    grouper: Option<Box<dyn Grouper>>,
}
impl Builder {
    pub fn new(group_type: Option<GroupType>) -> Self {
        let grouper = group_type.map(|t| -> Box<dyn Grouper> {
            match t {
                GroupType::Namespace => Box::new(GrouperByNamespace),
                GroupType::Object => Box::new(GrouperByObject),
            }
        });
        let report = Report {
            grouped_by: group_type,
            ..Default::default()
        };
        Self { report, grouper }
    }

    pub fn add_info(&mut self, object_name: &str, info: fl::Info) {
        let ctx = GroupCtx { object_name, info };
        self.report.overall.add(&ctx.info);

        let Some(grouper) = &self.grouper else {
            return;
        };
        let group = grouper.group_mut(&mut self.report.groups, &ctx);
        group.add(&ctx.info);
    }

    pub fn done(self) -> Report {
        self.report
    }
}

/// Full progress report
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// If the report has group type, this is the type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grouped_by: Option<GroupType>,
    /// If the report has group type, the stats of groups
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub groups: BTreeMap<String, Stats>,
    /// Overall stats
    pub overall: Stats,
}
impl Report {
    pub fn diff(&mut self, other: &Report) -> cu::Result<()> {
        if self.grouped_by != other.grouped_by {
            cu::bail!("cannot diff 2 reports that are grouped by different types");
        }
        self.overall.diff(Some(&other.overall));
        let group_names = self
            .groups
            .keys()
            .chain(other.groups.keys())
            .unique()
            .map(|x| x.to_string())
            .collect_vec();
        for group in group_names {
            let other_group = other.groups.get(&group);
            let self_group = self.groups.entry(group).or_default();
            self_group.diff(other_group);
        }
        Ok(())
    }
}

/// Stats collector
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    /// Total stats
    pub total: StatsEntry,
    /// Stats broken down by decompilation status
    pub by_status: fl::StatusMap<StatsEntry>,
}
impl Stats {
    fn add(&mut self, info: &fl::Info) {
        self.total.add(info);
        self.by_status.entry(info.status).or_default().add(info);
    }
    fn diff(&mut self, other: Option<&Stats>) {
        match other {
            Some(other) => {
                self.total.diff(Some(&other.total));
                for status in fl::Status::all() {
                    if self.by_status.contains_key(status) || other.by_status.contains_key(status) {
                        let self_by_status = self.by_status.entry(*status).or_default();
                        let other_by_status = other.by_status.get(status);
                        self_by_status.diff(other_by_status);
                    }
                }
            }
            None => {
                self.total.diff(None);
                for entry in self.by_status.values_mut() {
                    entry.diff(None);
                }
            }
        }
    }
    /// Return true if this `Stats` has been diffed and there was a change
    pub fn changed(&self) -> bool {
        self.total.changed() || self.by_status_changed()
    }

    pub fn by_status_changed(&self) -> bool {
        self.by_status.values().any(|x| x.changed())
    }
}

/// One entry in the stats
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatsEntry {
    /// Number of functions
    pub count: u32,
    /// Function sizes in bytes
    pub bytes: u32,
    /// If the entry is diffed against another, the diff result
    ///
    /// The diff is `self - other`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<DiffEntry>,
}
impl Default for &StatsEntry {
    fn default() -> Self {
        static DEFAULT: StatsEntry = StatsEntry {
            count: 0,
            bytes: 0,
            diff: None,
        };
        &DEFAULT
    }
}
impl StatsEntry {
    fn add(&mut self, info: &fl::Info) {
        self.count += 1;
        self.bytes += info.size;
    }
    fn diff(&mut self, other: Option<&Self>) {
        match other {
            Some(other) => {
                let count_diff = self.count as i32 - other.count as i32;
                let bytes_diff = self.bytes as i32 - other.bytes as i32;
                self.diff = Some(DiffEntry {
                    count: count_diff,
                    bytes: bytes_diff,
                })
            }
            None => {
                self.diff = Some(DiffEntry {
                    count: self.count as i32,
                    bytes: self.bytes as i32,
                })
            }
        }
    }
    /// Return true if this `StatsEntry` has been diffed and there was a change
    pub fn changed(&self) -> bool {
        self.diff
            .as_ref()
            .is_some_and(|x| x.count != 0 && x.bytes != 0)
    }

    /// Get comparison (the other side of diff)
    pub fn comparison(&self) -> Self {
        match &self.diff {
            Some(diff) => Self {
                count: (self.count as i32 - diff.count) as u32,
                bytes: (self.bytes as i32 - diff.bytes) as u32,
                diff: None,
            },
            None => self.clone(),
        }
    }
}

/// One diff entry
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffEntry {
    /// Diff of number of functions
    pub count: i32,
    /// Diff of function sizes in bytes
    pub bytes: i32,
}

trait Grouper {
    /// Retrieve or create the stats of a group
    fn group_mut<'map>(
        &self,
        groups: &'map mut BTreeMap<String, Stats>,
        ctx: &GroupCtx<'_>,
    ) -> &'map mut Stats {
        let key = self.key_from_ctx(ctx);
        // FIXME: skip the extra look up (needs polonius)
        if !groups.contains_key(key) {
            return groups.entry(key.to_string()).or_default();
        }
        let group = groups.get_mut(key);
        unsafe { group.unwrap_unchecked() }
    }

    fn key_from_ctx<'a, 'b>(&self, ctx: &'a GroupCtx<'b>) -> &'a str
    where
        'b: 'a;
}

struct GrouperByObject;
impl Grouper for GrouperByObject {
    fn key_from_ctx<'a, 'b>(&self, ctx: &'a GroupCtx<'b>) -> &'a str
    where
        'b: 'a,
    {
        ctx.object_name
    }
}
struct GrouperByNamespace;
impl Grouper for GrouperByNamespace {
    fn key_from_ctx<'a, 'b>(&self, ctx: &'a GroupCtx<'b>) -> &'a str
    where
        'b: 'a,
    {
        demangle::get_top_namespace(ctx.info.name()).unwrap_or("global")
    }
}

#[derive(Debug, Clone)]
struct GroupCtx<'a> {
    object_name: &'a str,
    info: fl::Info,
}
