import { describe, expect, it } from "vitest";

import { parsePreferenceRuleDisplayName } from "./preferenceRuleDisplay";

describe("parsePreferenceRuleDisplayName", () => {
  it("parses a numeric feature interval", () => {
    expect(
      parsePreferenceRuleDisplayName(
        "Learned feature evidence: profile:profile-v1:page_similarity_p90:between:0.7000:0.8000",
      ),
    ).toEqual({
      profileVersion: "profile-v1",
      feature: "page_similarity_p90",
      operator: "between",
      min: 0.7,
      max: 0.8,
    });
  });

  it("parses an equality feature whose key contains colons", () => {
    expect(
      parsePreferenceRuleDisplayName(
        "Learned feature evidence: profile:profile-v1:theme:example-theme:eq:1",
      ),
    ).toEqual({
      profileVersion: "profile-v1",
      feature: "theme:example-theme",
      operator: "eq",
      value: 1,
    });
  });

  it("returns null for unknown or malformed formats", () => {
    expect(
      parsePreferenceRuleDisplayName("User-created preference rule"),
    ).toBeNull();
    expect(
      parsePreferenceRuleDisplayName(
        "Learned feature evidence: profile:profile-v1:page_count:contains:10",
      ),
    ).toBeNull();
    expect(
      parsePreferenceRuleDisplayName(
        "Learned feature evidence: profile:profile-v1:page_count:eq:not-a-number",
      ),
    ).toBeNull();
  });
});
