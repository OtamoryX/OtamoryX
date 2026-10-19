// @vitest-environment jsdom

import { mount, flushPromises } from "@vue/test-utils";
import { createPinia } from "pinia";
import { QueryClient, VueQueryPlugin } from "@tanstack/vue-query";
import { nextTick } from "vue";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import RandomCarousel from "@/components/library/RandomCarousel.vue";

const apiMocks = vi.hoisted(() => ({
  getRandomArchiveSession: vi.fn(),
  reportRandomRecommendationImpressions: vi.fn(),
}));

vi.mock("@/utils/api", () => ({
  getRandomArchiveSession: apiMocks.getRandomArchiveSession,
  reportRandomRecommendationImpressions:
    apiMocks.reportRandomRecommendationImpressions,
}));

class FakeIntersectionObserver {
  static instances: FakeIntersectionObserver[] = [];

  private readonly callback: IntersectionObserverCallback;
  private readonly elements: Element[] = [];

  constructor(callback: IntersectionObserverCallback) {
    this.callback = callback;
    FakeIntersectionObserver.instances.push(this);
  }

  observe(element: Element) {
    this.elements.push(element);
  }

  unobserve() {}

  disconnect() {}

  takeRecords(): IntersectionObserverEntry[] {
    return [];
  }

  trigger(positions: number[]) {
    const entries = this.elements
      .filter((element) =>
        positions.includes(Number((element as HTMLElement).dataset.position)),
      )
      .map(
        (target) =>
          ({
            isIntersecting: true,
            intersectionRatio: 1,
            target,
          }) as IntersectionObserverEntry,
      );
    this.callback(entries, this as unknown as IntersectionObserver);
  }
}

const session = {
  sessionId: "session-1",
  algorithmVariant: "weighted-v1",
  archives: Array.from({ length: 6 }, (_, index) => ({
    id: `archive-${index}`,
    title: `Archive ${index}`,
  })),
};

const mountCarousel = () =>
  mount(RandomCarousel, {
    global: {
      plugins: [
        createPinia(),
        [
          VueQueryPlugin,
          {
            queryClient: new QueryClient({
              defaultOptions: { queries: { retry: false } },
            }),
          },
        ],
      ],
      stubs: { ArchiveThumbnailCard: true },
    },
  });

describe("RandomCarousel impressions", () => {
  beforeEach(() => {
    apiMocks.getRandomArchiveSession.mockResolvedValue(session);
    apiMocks.reportRandomRecommendationImpressions.mockResolvedValue(undefined);
    FakeIntersectionObserver.instances = [];
    vi.stubGlobal("IntersectionObserver", FakeIntersectionObserver);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.clearAllMocks();
  });

  it("reports each card that entered the viewport exactly once", async () => {
    const wrapper = mountCarousel();
    await flushPromises();
    await nextTick();
    await nextTick();

    const observer = FakeIntersectionObserver.instances.at(-1);
    expect(observer).toBeDefined();

    observer?.trigger([0, 2, 4]);
    await new Promise((resolve) => setTimeout(resolve, 450));

    expect(
      apiMocks.reportRandomRecommendationImpressions,
    ).toHaveBeenCalledTimes(1);
    expect(
      apiMocks.reportRandomRecommendationImpressions,
    ).toHaveBeenCalledWith("session-1", [0, 2, 4]);

    observer?.trigger([0, 2]);
    await new Promise((resolve) => setTimeout(resolve, 450));
    expect(
      apiMocks.reportRandomRecommendationImpressions,
    ).toHaveBeenCalledTimes(1);

    wrapper.unmount();
  });

  it("does not report when the carousel stays on the first screen", async () => {
    const wrapper = mountCarousel();
    await flushPromises();
    await nextTick();
    await nextTick();

    await new Promise((resolve) => setTimeout(resolve, 450));
    expect(
      apiMocks.reportRandomRecommendationImpressions,
    ).not.toHaveBeenCalled();

    wrapper.unmount();
  });
});
