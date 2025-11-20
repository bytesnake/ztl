use regex::Regex;
use jiff::{Span, civil::Date, Zoned, tz::TimeZone, SpanRound, Unit};
use std::cmp::{Ord, Ordering, PartialOrd};
use indexmap::{IndexMap, IndexSet};
//use anyhow::{Result, Context};

use ztl_base::{notes::Notes, error::Result};
use crate::{Config, commands::result::{Output, ScheduleEntry}};

#[derive(Debug)]
pub enum State {
    Inactive,
    Due(Span),
    OverDue(Span),
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (State::Inactive, State::Inactive) => Ordering::Equal,
            (State::Inactive, _) => Ordering::Less,
            (State::Due(_), State::Inactive) => Ordering::Greater,
            (State::Due(a), State::Due(b)) => a.compare(b).unwrap(),
            (State::Due(_), State::OverDue(_)) => Ordering::Less,
            (State::OverDue(a), State::OverDue(b)) => b.compare(a).unwrap(),
            (State::OverDue(_), _) => Ordering::Greater,
        }
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (State::Inactive, State::Inactive) => true,
            (State::Due(a), State::Due(b)) => a.compare(b).unwrap() == Ordering::Equal,
            (State::OverDue(a), State::OverDue(b)) => a.compare(b).unwrap() == Ordering::Equal,
            _ => false,
        }
    }
}

impl Eq for State {}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Inactive => write!(f, "inactive"),
            State::Due(span) => {
                let span = span.round(SpanRound::new().largest(Unit::Day).smallest(Unit::Hour).days_are_24_hours()).unwrap();
                write!(f, "{}", format!("due:{:#}", span))
            },
            State::OverDue(span) => {
                let span = span.round(SpanRound::new().largest(Unit::Day).smallest(Unit::Hour).days_are_24_hours()).unwrap();
                write!(f, "{}", format!("overdue:{:#}", span))
            },
        }
    }
}

#[derive(Debug)]
pub enum Schedule {
    MinMax { min: Span, max: Option<Span> },
    Every(Span),
    Until { until: Span, count: Option<u32> },
}

impl Schedule {
    pub fn to_state(self, created: Option<Date>, mut instances: Vec<Date>) -> State {
        let now = Zoned::now();
        instances.sort();

        match self {
            Schedule::MinMax { min, max } => {
                // join created at and instances
                if let Some(created) = created {
                    instances.push(created);
                }

                instances.sort();
                let latest = instances.pop().unwrap().to_zoned(TimeZone::system()).unwrap();

                if &latest + min >= now {
                    return State::Inactive;
                } else {
                    if let Some(max) = max {
                        if &latest + max < now {
                            return State::OverDue(&now - &(&latest + max));
                        } else {
                            return State::Due(&(&latest + max) - &now);
                        }
                    }

                    return State::Due(&latest - &(&now - min));
                }
            },
            Schedule::Every(interval) => {
                instances.sort();
                let latest = instances.pop().unwrap().to_zoned(TimeZone::system()).unwrap();

                if &latest + interval > now {
                    //return State::Due(&latest - &(&now - interval));
                    return State::Inactive;
                } else {
                    return State::OverDue(&now - &(&latest + interval));
                }
            },
            Schedule::Until { until, count } => {
                instances.sort();
                let created = created.unwrap_or(instances[0].clone()).to_zoned(TimeZone::system()).unwrap();

                if let Some(count) = count {
                    // mark as inactive if expired or number of occurences fulfilled
                    if now >= &created + until || instances.len() as u32 >= count {
                        return State::Inactive;
                    }

                    // calculate when the next event should happen
                    //let frac = (count as f32 + 1.0) / instances.len() as f32;
                    let span = until.to_duration(&now).unwrap() * (instances.len() + 1) as i32 / count as i32;

                    //dbg!(&span);
                    //dbg!(&until.to_duration(&now).unwrap());

                    let span = Span::try_from(span).unwrap();

                    if now < &created + span {
                        return State::Due(&(&created + span) - &now);
                    } else {
                        return State::OverDue(&now - &(&created + span));
                    }
                } else {
                    if now >= &created + until {
                        return State::OverDue(&now - &(&created + until));
                    } else {
                        return State::Due(&(&created + until) - &now);
                    }
                }
            }
        }
    }
}
// Top-level parser that tries all three
pub fn parse_schedule(input: &str) -> Schedule {
    let kv_re = Regex::new(r"([a-zA-Z]+)\s*=\s*(\w+)\s*").unwrap();
let pairs: Vec<_> = kv_re
        .captures_iter(input)
        .map(|cap| (cap[1].to_string(), cap[2].to_string()))
        .collect();

    let mut min: Option<Span> = None;
    let mut max: Option<Span> = None;
    let mut every: Option<Span> = None;
    let mut until: Option<Span> = None;
    let mut count: Option<u32> = None;

    for (key, val) in pairs {
        match key.as_str() {
            "min" => min = Some(val.parse().unwrap()),
            "max" => max = Some(val.parse().unwrap()),
            "every" => every = Some(val.parse().unwrap()),
            "until" => until = Some(val.parse().unwrap()),
            "count" => count = Some(val.parse::<u32>().unwrap()),
            _ => {}
        }
    }

    match (min, max, every, until, count) {
        (Some(min), max, _, _, _) => Schedule::MinMax { min, max },
        (None, _, Some(every), None, None) => Schedule::Every(every),
        (None, _, None, Some(until), count) => Schedule::Until { until, count },
        _ => panic!(""),
    }
}

pub(crate) fn schedule(cfg: Config) -> Result<Output> {
    let notes = Notes::from_cache(&cfg.ztl_root())?;
    let mut scheds = Vec::new();

    // capture all "<schedule <field>=<value>>" in sources
    let re = regex::Regex::new(r"<schedule\s*(?:([\s\w=]*))\s+\/>").unwrap();
    for (note, directive) in notes.notes.values().filter_map(|x| re.captures(&x.html).map(|y| (x, y))) {
        use jiff::civil::Date;
        // parse headers into time stamp
        let re = regex::Regex::new(r"^((?:(\d{4}-\d{2}-\d{2})))").unwrap();
        let created = re.captures(&note.header);

        // get all instances from children notes
        let instances = note.children.iter()
            .filter_map(|x| notes.notes.get(x))
            .filter_map(|x| re.captures(&x.header))
            .map(|x| x.get(0).unwrap().as_str().parse().unwrap())
            .collect::<Vec<Date>>();

        // convert directive into absolute timestamps
        let created: Option<Date> = created.and_then(|x| x.get(0).map(|x| x.as_str().parse().unwrap()));

        //dbg!(&created, &instances);

        // we support three strategies for directives
        let directive = directive.get(0).unwrap().as_str();
        let sched = parse_schedule(directive);
        let state = sched.to_state(created, instances);
        scheds.push((note, state));
    }

    let mut processed = IndexMap::new();

    let names = vec!["micro", "meso", "macro"];
    let mut current;
    while scheds.len() != 0 {
        let keys = scheds.iter().map(|x| x.0.id.clone()).collect::<IndexSet<_>>();

        // split into two sets, those with still active children, and those with none
        // true if parent exists, and still in process
        // false if parent not exists or not contained in process
        (scheds, current) = scheds.into_iter()
            .partition(|x| x.0.parent.as_ref().map(|x| keys.contains(x)).unwrap_or(false));

        for (note, state) in current {
            // check if this is a leaf note, then distribute labels
            let label = if !note.children.iter().any(|x| keys.contains(x)) {
                let mut parent = note.parent.clone();
                let mut idx = 1;
                while let Some(p) = parent {
                    if !processed.contains_key(&p) {
                        break;
                    }

                    let node: &mut (State, String, bool) = processed.get_mut(&p).unwrap();
                    node.1 = names[idx].to_string();

                    if idx < names.len() {
                        idx += 1;
                    }

                    parent = notes.notes.get(&p).unwrap().parent.clone();
                }

                names[0].to_string()
            } else {
                String::new()
            };

            let pkey = match note.parent.as_ref() {
                Some(pkey) => pkey,
                _ => {
                    processed.insert(note.id.clone(), (state, label, true));
                    continue;
                },
            };

            // if this note has no scheduling parent, we can just add it 
            if !processed.contains_key(pkey) {
                processed.insert(note.id.clone(), (state, label, true));
                continue;
            }

            // check that the parent state is not over due and valid
            let state_parent = processed.get(pkey).unwrap();
            if !matches!(state_parent.0, State::OverDue(_)) && state_parent.2 {
                processed.insert(note.id.clone(), (state, label, true));
            } else {
                processed.insert(note.id.clone(), (state, label, false));
            }
        }
    }

    let mut processed = processed.into_iter().filter(|x| x.1.2).map(|x| (x.0, (x.1.0, x.1.1))).collect::<Vec<_>>();
    processed.sort_by(|x, y| x.1.0.cmp(&y.1.0));

    let res = processed.into_iter().map(|(id, (state, label))| {
        let note = notes.notes.get(&id).unwrap();

        ScheduleEntry {
            key: note.id.clone(), header: note.header.clone(), state: format!("{}", state), label }
    }).collect::<Vec<_>>();

    Ok(Output::Schedule(res))
}
