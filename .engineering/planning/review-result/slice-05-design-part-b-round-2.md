---
format: aep.planning-md/3
id: review-result:slice-05-design-part-b-round-2
kind: review-result
status: active
title: design critic, second reading of the generator story of slice 5
relations:
- reviews: story:unicode-gen-table-list-and-source-digests
revision: 1
---
approve

All five of my round-1 findings are closed at `16253c3`, and the revision opened no new blocker.

- **Blocker, `check.rs` hard-coded to one table:** closed. The revised prefactor builds `EQUAL` from the number of lines of the new list file `crates/unicode-gen/tables.txt`. Its copy now holds `SOURCE`, the list, and every source and table the list names. The new test `a_table_added_to_a_copy_of_the_list_is_written_and_checked` checks that a second line prints `2 table(s)`. The acceptance adds `grep -c 'unicode-tables: 1 table(s)' crates/unicode-gen/tests/check.rs`, which prints `0`. The case story and the `v` story both say `check.rs` takes their tables from the list and is not written by them.
- **Readers and writers with no caller:** closed. The `UnicodeData.txt` reader and the pair writer are now in the case story's Change and Writes. The sequence reader and writer are in the `v` story's. Each is tested through the story's own tables. The prefactor keeps only the range reader that `Zs` uses, has no `expect(dead_code)` and drops `tests/ucd.rs`.
- **`v` story and the identifiers story:** closed. The graph now has the edge from `story:regexp-v-flag-class-sets-and-properties-of-strings` to `story:identifiers-and-group-names-outside-ascii` (depends_on). The `v` story's Change says its base holds the identifiers story.
- **Normalization's table count:** closed. It now reads "three more than on the base of the unit".
- **`is_id_start` and `is_id_continue`:** closed. The property story now tests them in `id_start_and_id_continue_answer_as_the_data_file`.

What I read: the revised prefactor story in full; the changed passages of the case, normalization, property-escape and `v` stories; the graph's edges from the `v` story; `aep plan artifact validate` (valid, the same one notice as before about a story outside this set); and the first line of each vendored data file.

What I could not establish: the prefactor's reader now rejects a file "whose first line names a Unicode version other than `17.0.0`". `ucd/emoji/emoji-data.txt` starts with `# emoji-data.txt`, which names no version, and the property story reads that file through this reader. Taken literally the check passes such a file, but the story does not say so. This is a question of how to word the acceptance, which is outside my lane.

```findings
[]
```
