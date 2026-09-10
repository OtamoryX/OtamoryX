export type ParsedPreferenceRule =
  | {
      profileVersion: string;
      feature: string;
      operator: "between";
      min: number;
      max: number;
    }
  | {
      profileVersion: string;
      feature: string;
      operator: "eq";
      value: number;
    };

const LEARNED_FEATURE_PREFIX = "Learned feature evidence:";
const NUMERIC_VALUE_PATTERN = /^[+-]?(?:\d+(?:\.\d*)?|\.\d+)$/;

const parseNumericValue = (value: string): number | null => {
  if (!NUMERIC_VALUE_PATTERN.test(value)) return null;

  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
};

/**
 * Parses the persisted name of a learned profile-feature preference rule.
 *
 * Feature keys may contain colons, so the operator and its values are located
 * from the end of the serialized condition instead of using fixed positions.
 */
export function parsePreferenceRuleDisplayName(
  input: string,
): ParsedPreferenceRule | null {
  if (typeof input !== "string") return null;

  const name = input.trim();
  if (!name.startsWith(LEARNED_FEATURE_PREFIX)) return null;

  const segments = name.slice(LEARNED_FEATURE_PREFIX.length).trim().split(":");
  if (segments.some((segment) => segment.length === 0)) return null;
  if (segments[0] !== "profile" || segments.length < 5) return null;

  const profileVersion = segments[1];
  if (!profileVersion) return null;

  const betweenOperatorIndex = segments.length - 3;
  if (segments[betweenOperatorIndex] === "between") {
    const feature = segments.slice(2, betweenOperatorIndex).join(":");
    const min = parseNumericValue(segments[segments.length - 2]);
    const max = parseNumericValue(segments[segments.length - 1]);
    if (!feature || min === null || max === null || min > max) return null;

    return {
      profileVersion,
      feature,
      operator: "between",
      min,
      max,
    };
  }

  const equalsOperatorIndex = segments.length - 2;
  if (segments[equalsOperatorIndex] !== "eq") return null;

  const feature = segments.slice(2, equalsOperatorIndex).join(":");
  const value = parseNumericValue(segments[segments.length - 1]);
  if (!feature || value === null) return null;

  return {
    profileVersion,
    feature,
    operator: "eq",
    value,
  };
}
