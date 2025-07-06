use semver::{Version, VersionReq};
use std::str::FromStr;
use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub enum VersionBound {
    Inclusive(Version),
    Exclusive(Version),
    Unbounded,
}

#[derive(Debug, PartialEq, Clone)]
pub enum VersionConstraint {
    Bracketed(VersionBound, VersionBound),
    Semver(VersionReq),
}

impl FromStr for VersionConstraint {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        // Try parsing as a semver::VersionReq first
        if let Ok(req) = VersionReq::from_str(s) {
            return Ok(VersionConstraint::Semver(req));
        }

        // If semver::VersionReq parsing fails, try the bracketed format
        if s.starts_with('[') || s.starts_with('(') {
            let parts: Vec<&str> = s[1..s.len() - 1].split(',').collect();
            if parts.len() != 2 {
                return Err("Expected `[min, max)` or `(min, max]` format".into());
            }

            let min_bound = parse_bound(parts[0], s.starts_with('['))?;
            let max_bound = parse_bound(parts[1], s.ends_with(']'))?;

            Ok(VersionConstraint::Bracketed(min_bound, max_bound))
        } else {
            // If neither matches, it's an invalid format
            Err(format!("Invalid version constraint format: {}", s))
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
        match self {
            VersionConstraint::Bracketed(min, max) => match (min, max) {
                (VersionBound::Inclusive(min), VersionBound::Unbounded) => write!(f, ">={}", min),
                (VersionBound::Exclusive(min), VersionBound::Unbounded) => write!(f, ">{}", min),
                (VersionBound::Inclusive(min), VersionBound::Inclusive(max)) => write!(f, "[{}, {}]", min, max),
                (VersionBound::Exclusive(min), VersionBound::Inclusive(max)) => write!(f, "({}, {}]", min, max),
                (VersionBound::Inclusive(min), VersionBound::Exclusive(max)) => write!(f, "[{}, {})", min, max),
                (VersionBound::Exclusive(min), VersionBound::Exclusive(max)) => write!(f, "({}, {})", min, max),
                _ => write!(f, "any"), // Should not happen with current parsing
            },
            VersionConstraint::Semver(req) => write!(f, "{}", req),
        }
    }
}

impl VersionConstraint {
    pub fn matches(&self, version: &Version) -> bool {
        match self {
            VersionConstraint::Bracketed(min, max) => {
                let min_ok = match min {
                    VersionBound::Inclusive(v) => version >= v,
                    VersionBound::Exclusive(v) => version > v,
                    VersionBound::Unbounded => true,
                };

                let max_ok = match max {
                    VersionBound::Inclusive(v) => version <= v,
                    VersionBound::Exclusive(v) => version < v,
                    VersionBound::Unbounded => true,
                };

                min_ok && max_ok
            }
            VersionConstraint::Semver(req) => req.matches(version),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use semver::Version;

    #[test]
    fn test_parse_version_constraint_bracketed_inclusive_unbounded() {
        let constraint: VersionConstraint = "[1.0.2-f,)".parse().unwrap();
        assert_eq!(
            constraint,
            VersionConstraint::Bracketed(
                VersionBound::Inclusive(Version::parse("1.0.2-f").unwrap()),
                VersionBound::Unbounded,
            )
        );
        assert!(constraint.matches(&Version::parse("1.0.2-f").unwrap()));
        assert!(constraint.matches(&Version::parse("1.0.3").unwrap()));
        assert!(!constraint.matches(&Version::parse("1.0.1").unwrap()));
    }

    #[test]
    fn test_parse_version_constraint_bracketed_exclusive_inclusive() {
        let constraint: VersionConstraint = "(1.0.0, 2.0.0]".parse().unwrap();
        assert_eq!(
            constraint,
            VersionConstraint::Bracketed(
                VersionBound::Exclusive(Version::parse("1.0.0").unwrap()),
                VersionBound::Inclusive(Version::parse("2.0.0").unwrap()),
            )
        );
        assert!(constraint.matches(&Version::parse("1.0.1").unwrap()));
        assert!(constraint.matches(&Version::parse("2.0.0").unwrap()));
        assert!(!constraint.matches(&Version::parse("1.0.0").unwrap()));
        assert!(!constraint.matches(&Version::parse("2.0.1").unwrap()));
    }

    #[test]
    fn test_parse_version_constraint_semver_greater_than_or_equal() {
        let constraint: VersionConstraint = ">=1.2.1".parse().unwrap();
        assert_eq!(
            constraint,
            VersionConstraint::Semver(VersionReq::from_str(">=1.2.1").unwrap())
        );
        assert!(constraint.matches(&Version::parse("1.2.1").unwrap()));
        assert!(constraint.matches(&Version::parse("1.2.2").unwrap()));
        assert!(!constraint.matches(&Version::parse("1.2.0").unwrap()));
    }

    #[test]
    fn test_parse_version_constraint_semver_caret_operator() {
        let constraint: VersionConstraint = "^1.2.3".parse().unwrap();
        assert_eq!(
            constraint,
            VersionConstraint::Semver(VersionReq::from_str("^1.2.3").unwrap())
        );
        assert!(constraint.matches(&Version::parse("1.2.3").unwrap()));
        assert!(constraint.matches(&Version::parse("1.2.4").unwrap()));
        assert!(constraint.matches(&Version::parse("1.9.9").unwrap()));
        assert!(!constraint.matches(&Version::parse("2.0.0").unwrap()));
    }

    #[test]
    fn test_parse_version_constraint_semver_tilde_operator() {
        let constraint: VersionConstraint = "~1.2.3".parse().unwrap();
        assert_eq!(
            constraint,
            VersionConstraint::Semver(VersionReq::from_str("~1.2.3").unwrap())
        );
        assert!(constraint.matches(&Version::parse("1.2.3").unwrap()));
        assert!(constraint.matches(&Version::parse("1.2.4").unwrap()));
        assert!(!constraint.matches(&Version::parse("1.3.0").unwrap()));
    }

    #[test]
    fn test_parse_version_constraint_semver_exact() {
        let constraint: VersionConstraint = "=1.0.0".parse().unwrap();
        assert_eq!(
            constraint,
            VersionConstraint::Semver(VersionReq::from_str("=1.0.0").unwrap())
        );
        assert!(constraint.matches(&Version::parse("1.0.0").unwrap()));
        assert!(!constraint.matches(&Version::parse("1.0.1").unwrap()));
    }

    #[test]
    fn test_parse_version_constraint_semver_range() {
        let constraint: VersionConstraint = ">1.0.0, <2.0.0".parse().unwrap();
        assert_eq!(
            constraint,
            VersionConstraint::Semver(VersionReq::from_str(">1.0.0, <2.0.0").unwrap())
        );
        assert!(constraint.matches(&Version::parse("1.0.1").unwrap()));
        assert!(!constraint.matches(&Version::parse("1.0.0").unwrap()));
        assert!(!constraint.matches(&Version::parse("2.0.0").unwrap()));
    }

    #[test]
    fn test_parse_version_constraint_invalid_format() {
        let result: Result<VersionConstraint, _> = "invalid-version".parse();
        assert!(result.is_err());
    }
}
