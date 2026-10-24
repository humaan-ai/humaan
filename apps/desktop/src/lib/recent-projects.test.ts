import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseRecentProjects, rememberProject } from './recent-projects';
test('recent projects tolerate corrupt storage and deduplicate newest entries', () => {
  assert.deepEqual(parseRecentProjects('{'), []);
  assert.deepEqual(parseRecentProjects('{}'), []);
  assert.deepEqual(parseRecentProjects(JSON.stringify([null, {root:'/a'}, {root:'/a',name:'Old',openedAt:1},{root:'/a',name:'New',openedAt:2}])), [{root:'/a',name:'New',openedAt:2}]);
});
test('reopening updates names and recency while capping history at twelve', () => {
  let entries = Array.from({length: 12}, (_, i) => ({root:`/${i}`,name:`Project ${i}`,openedAt:12-i}));
  entries = rememberProject(entries, {root:'/5',project:{id:'5',name:'Renamed'}}, 20);
  assert.equal(entries.length, 12);
  assert.deepEqual(entries[0], {root:'/5',name:'Renamed',openedAt:20});
  entries = rememberProject(entries, {root:'/new',project:{id:'new',name:'New'}}, 21);
  assert.equal(entries.length, 12);
  assert.equal(entries.at(-1)?.root, '/10');
});
