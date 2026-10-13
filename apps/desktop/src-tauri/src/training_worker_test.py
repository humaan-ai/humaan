import copy
import pathlib
import unittest
from simulation_worker_test import fixture

namespace = {'__name__': 'humaan_training_test'}
folder = pathlib.Path(__file__).parent
exec((folder/'simulation_worker.py').read_text().split("if __name__ == '__main__':")[0] + '\n' + (folder/'training_worker.py').read_text(), namespace)
train = namespace['train']


def experiment():
    return {'id': '123', 'robot': fixture()['robot'], 'environment': {'id': '1', 'name': 'Bench', 'kind': 'fixed-base-floor', 'duration': .3, 'gravity': 0}, 'task': {'id': '2', 'name': 'Reach', 'kind': 'joint-reach', 'targetFraction': .7, 'tolerance': .1}, 'config': {'seed': 42, 'iterations': 2, 'population': 4}}


class TrainingTests(unittest.TestCase):
    def test_real_training_is_reproducible_preserves_inputs_and_evaluates_held_out(self):
        payload = experiment()
        before = copy.deepcopy(payload)
        result = train(payload)
        repeat = train(payload)
        self.assertEqual(before, payload)
        self.assertEqual(result['policy'], repeat['policy'])
        self.assertEqual(result['evaluation'], repeat['evaluation'])
        self.assertLessEqual(result['history'][-1]['loss'], result['history'][0]['loss'])
        self.assertEqual(len(result['history']), 3)
        self.assertEqual(result['evaluation']['seeds'], list(range(1000000, 1000005)))
        self.assertEqual(result['policy']['jointIds'], ['joint'])
        self.assertEqual(len(result['frames']), 7)
        self.assertGreater(result['frames'][-1]['joints']['joint'], result['frames'][0]['joints']['joint'])
        self.assertTrue(0 <= result['evaluation']['successRate'] <= 1)

    def test_comparison_protocol_changes_only_for_evaluation_semantics(self):
        payload = experiment()
        first = train(payload)
        payload['task']['name'] = 'Renamed'
        payload['robot']['name'] = 'Another design'
        payload['config']['seed'] = 123
        second = train(payload)
        self.assertEqual(first['comparisonKey'], second['comparisonKey'])
        payload['task']['tolerance'] = .05
        self.assertNotEqual(first['comparisonKey'], train(payload)['comparisonKey'])

    def test_rejects_unsupported_training_joints(self):
        payload = experiment()
        payload['robot']['joints'][0]['type'] = 'continuous'
        with self.assertRaisesRegex(ValueError, 'finite limits'): train(payload)
        payload = experiment()
        payload['robot']['joints'][0]['type'] = 'fixed'
        with self.assertRaisesRegex(ValueError, 'actuated joints'): train(payload)

if __name__ == '__main__': unittest.main()
