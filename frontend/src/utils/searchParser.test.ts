import { describe, expect, it } from 'vitest'

import { parseSearchQuery, toSearchParams } from './searchParser'

describe('parseSearchQuery', () => {
  it('returns an empty parsed result for blank input', () => {
    expect(parseSearchQuery('   ')).toEqual({
      query: '',
      tags: [],
    })
  })

  it('separates free-text query parts from tags', () => {
    expect(parseSearchQuery('some title, artist:john, genre:action')).toEqual({
      query: 'some title',
      tags: ['artist:john', 'genre:action'],
    })
  })

  it('keeps malformed tag-like segments in the text query', () => {
    expect(parseSearchQuery('title, artist:, :broken, plain')).toEqual({
      query: 'title, artist:, :broken, plain',
      tags: [],
    })
  })
})

describe('toSearchParams', () => {
  it('omits empty query and tags from the API params', () => {
    expect(toSearchParams({ query: '', tags: [] }, 2, 40)).toEqual({
      pageNumb: 2,
      pageSize: 40,
    })
  })

  it('maps parsed search values into API params', () => {
    expect(
      toSearchParams(
        {
          query: 'some title',
          tags: ['artist:john'],
        },
        1,
        24,
      ),
    ).toEqual({
      pageNumb: 1,
      pageSize: 24,
      query: 'some title',
      tags: ['artist:john'],
    })
  })
})
