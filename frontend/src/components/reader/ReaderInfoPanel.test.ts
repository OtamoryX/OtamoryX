// @vitest-environment jsdom

import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { defineComponent, h } from "vue";
import ReaderInfoPanel from "@/components/reader/ReaderInfoPanel.vue";

const BaseSidePanelStub = defineComponent({
  props: { show: Boolean },
  setup(_, { slots }) {
    return () => h("div", { "data-testid": "side-panel" }, slots.default?.());
  },
});

const ConfirmModalStub = defineComponent({
  props: { show: Boolean },
  setup(props) {
    return () =>
      props.show ? h("div", { "data-testid": "delete-confirm" }) : null;
  },
});

const baseProps = {
  show: true,
  currentPage: 1,
  totalPages: 1,
  displayModeLabel: "适应",
  readingModeLabel: "单页",
  pluginOptions: [],
  pluginsLoading: false,
  pluginExecuting: false,
  pluginExecutionSummary: null,
  deleteError: null as string | null,
  deleteLoading: false,
  translationRetrying: false,
  translationRetryMessage: null,
  ehentaiCandidates: [],
  ehentaiSearching: false,
  ehentaiSearchError: null,
  nhentaiCandidates: [],
  nhentaiSearching: false,
  nhentaiSearchError: null,
};

const mountPanel = (overrides: Partial<typeof baseProps> = {}) =>
  mount(ReaderInfoPanel, {
    props: { ...baseProps, ...overrides },
    global: {
      stubs: {
        BaseSidePanel: BaseSidePanelStub,
        ConfirmModal: ConfirmModalStub,
      },
    },
  });

describe("ReaderInfoPanel delete action", () => {
  it("shows delete errors as an alert", () => {
    const wrapper = mountPanel({ deleteError: "删除失败，请稍后重试" });
    const alert = wrapper.get('[role="alert"]');

    expect(alert.text()).toBe("删除失败，请稍后重试");
    expect(alert.attributes("aria-live")).toBe("polite");
  });

  it("disables deletion while a request is pending", () => {
    const wrapper = mountPanel({ deleteLoading: true });
    const deleteButton = wrapper.get("section:last-of-type button");

    expect((deleteButton.element as HTMLButtonElement).disabled).toBe(true);
    expect(deleteButton.text()).toContain("删除中...");
  });

  it("opens the confirmation modal in the normal state", async () => {
    const wrapper = mountPanel();
    const deleteButton = wrapper.get("section:last-of-type button");

    expect((deleteButton.element as HTMLButtonElement).disabled).toBe(false);
    await deleteButton.trigger("click");

    expect(wrapper.find('[data-testid="delete-confirm"]').exists()).toBe(true);
  });
});
