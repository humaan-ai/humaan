import { test } from 'node:test';
import assert from 'node:assert/strict';
import { comparableRuns, type TrainingRun } from './model';
test('comparison excludes failed runs and mismatched evaluation protocols', () => {
  const runs = [
    { id:'1', status:'completed', comparisonKey:'same', versionId:'a' },
    { id:'2', status:'completed', comparisonKey:'same', versionId:'b' },
    { id:'3', status:'completed', comparisonKey:'different', versionId:'a' },
    { id:'4', status:'failed', comparisonKey:'same', versionId:'a' },
  ] as TrainingRun[];
  assert.deepEqual(comparableRuns(runs, { result: { comparisonKey:'same' } } as TrainingRun).map(r => r.id), ['1','2']);
  assert.deepEqual(comparableRuns(runs,null), []);
});
