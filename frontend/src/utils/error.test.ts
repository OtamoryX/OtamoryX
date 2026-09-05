import { describe, expect, it } from "vitest";
import { getApiErrorMessage } from "@/utils/error";

describe("getApiErrorMessage", () => {
  it("uses the first non-empty response message", () => {
    expect(
      getApiErrorMessage(
        { response: { data: { message: "  ", error: "删除失败，请稍后重试" } } },
        "操作失败",
      ),
    ).toBe("删除失败，请稍后重试");

    expect(
      getApiErrorMessage(
        { response: { data: { message: 500, error: "权限不足" } } },
        "操作失败",
      ),
    ).toBe("权限不足");
  });

  it("uses the fallback for an empty HTTP error response", () => {
    expect(
      getApiErrorMessage(
        { response: { status: 500 }, message: "Request failed with status code 500" },
        "删除失败，请稍后重试",
      ),
    ).toBe("删除失败，请稍后重试");
  });
});
