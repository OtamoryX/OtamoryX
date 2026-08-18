import { describe, expect, it } from "vitest";
import { localizeTrashReason } from "./trashReason";

describe("localizeTrashReason", () => {
  it("localizes persisted manual deletion reasons", () => {
    expect(
      localizeTrashReason(
        { reason: "user initiated archive deletion", ruleId: undefined, operationType: undefined },
        "手动删除",
      ),
    ).toBe("用户主动删除漫画");
  });

  it("describes the matched automatic rule without exposing its machine prefix", () => {
    expect(
      localizeTrashReason(
        { reason: "preference rule No gore matched", ruleId: "rule-1", operationType: undefined },
        "自动删除",
      ),
    ).toBe("命中偏好规则“No gore”");
  });

  it("uses a localized fallback for unknown reasons", () => {
    expect(
      localizeTrashReason(
        { reason: "unknown backend reason", ruleId: undefined, operationType: undefined },
        "手动删除",
      ),
    ).toBe("用户主动删除漫画");
  });
});
