use semver::{Version, VersionReq, Comparator, Op};
use std::str::FromStr;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum VersionBound {
    Inclusive(Version),
    Exclusive(Version),
    Unbounded,
}

#[derive(Debug, PartialEq)]
pub struct VersionConstraint {
    pub min: VersionBound,
    pub max: VersionBound,
}

impl FromStr for VersionConstraint {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        if s.starts_with('[') || s.starts_with('(') {
            let parts: Vec<&str> = s[1..s.len()-1].split(',').collect();
            if parts.len() != 2 {
                return Err("Expected `[min, max)` or `(min, max]` format".into());
            }

            let min_bound = parse_bound(parts[0], s.starts_with('['))?;
            let max_bound = parse_bound(parts[1], s.ends_with(']'))?;

            Ok(VersionConstraint {
                min: min_bound,
                max: max_bound,
            })
        } else {
            let version = Version::parse(s)
                .map_err(|e| format!("Invalid version: {}", e))?;

            Ok(VersionConstraint {
                min: VersionBound::Inclusive(version.clone()),
                max: VersionBound::Unbounded,
            })
        }
    }
}

fn parse_bound(s: &str, inclusive: bool) -> Result<VersionBound, String> {
    let s = s.trim();
    if s.is_empty() {
        Ok(VersionBound::Unbounded)
    } else {
        let version = Version::parse(s)
            .map_err(|e| format!("Invalid version: {}", e))?;

        if inclusive {
            Ok(VersionBound::Inclusive(version))
        } else {
            Ok(VersionBound::Exclusive(version))
        }
    }
}

impl fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (&self.min, &self.max) {
            (VersionBound::Inclusive(min), VersionBound::Unbounded) => write!(f, ">={}", min),
            (VersionBound::Exclusive(min), VersionBound::Unbounded) => write!(f, ">{}", min),
            (VersionBound::Inclusive(min), VersionBound::Inclusive(max)) => write!(f, "[{}, {}]", min, max),
            (VersionBound::Exclusive(min), VersionBound::Inclusive(max)) => write!(f, "({}, {}]", min, max),
            (VersionBound::Inclusive(min), VersionBound::Exclusive(max)) => write!(f, "[{}, {})", min, max),
            (VersionBound::Exclusive(min), VersionBound::Exclusive(max)) => write!(f, "({}, {})", min, max),
            _ => write!(f, "any"),
        }
    }
}

impl VersionConstraint {
    pub fn matches(&self, version: &Version) -> bool {
        let min_ok = match &self.min {
            VersionBound::Inclusive(v) => version >= v,
            VersionBound::Exclusive(v) => version > v,
            VersionBound::Unbounded => true,
        };

        let max_ok = match &self.max {
            VersionBound::Inclusive(v) => version <= v,
            VersionBound::Exclusive(v) => version < v,
            VersionBound::Unbounded => true,
        };

        min_ok && max_ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_constraint() {
        let constraint: VersionConstraint = "[1.0.2-f,)".parse().unwrap();
        assert_eq!(
            constraint,
            VersionConstraint {
                min: VersionBound::Inclusive(Version::parse("1.0.2-f").unwrap()),
                max: VersionBound::Unbounded,
            }
        );

        let constraint: VersionConstraint = "(1.0.0, 2.0.0]".parse().unwrap();
        assert_eq!(
            constraint,
            VersionConstraint {
                min: VersionBound::Exclusive(Version::parse("1.0.0").unwrap()),
                max: VersionBound::Inclusive(Version::parse("2.0.0").unwrap()),
            }
        );

        let constraint: VersionConstraint = "1.2.3".parse().unwrap();
        assert_eq!(
            constraint,
            VersionConstraint {
                min: VersionBound::Inclusive(Version::parse("1.2.3").unwrap()),
                max: VersionBound::Unbounded,
            }
        );
    }

    #[test]
    fn test_matches_version() {
        let constraint: VersionConstraint = "[15.0.0.f, 16.0.0)".parse().unwrap();

        assert!(constraint.matches(&Version::parse("15.5.1-beta.3").unwrap()));
        assert!(constraint.matches(&Version::parse("15.0.0.a").unwrap()));
        assert!(!constraint.matches(&Version::parse("14.0.1").unwrap()));
    }
}