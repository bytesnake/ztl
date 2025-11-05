use regex::Regex;
use jiff::{Span, civil::Date, Zoned, tz::TimeZone, SpanRound, Unit};
use std::cmp::{Ord, Ordering, PartialOrd};

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

                    dbg!(&span);
                    dbg!(&until.to_duration(&now).unwrap());

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
