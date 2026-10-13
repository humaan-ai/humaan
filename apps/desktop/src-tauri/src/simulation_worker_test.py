import copy
import unittest
from simulation_worker import simulate


def fixture():
    pose = {'xyz': [0, 0, 0], 'rpy': [0, 0, 0]}
    shape = {'kind': 'box', 'size': [.1,.1,.1], 'origin': pose}
    def link(id):
        return {'id': id, 'name': id, 'mass': 1, 'inertia': [.01,0,0,.01,0,.01], 'inertial': pose, 'collisions': [shape]}
    return {'robot': {'links': [link('base'),link('arm')], 'joints': [{'id': 'joint', 'name': 'joint', 'parent': 'base', 'child': 'arm', 'type': 'revolute', 'axis': [0,0,1], 'lower': -1, 'upper': 1, 'effort': 10, 'velocity': 2, 'origin': {'xyz': [0,0,.2], 'rpy': [.2,.3,.4]}}], 'assets': {}}, 'config': {'duration': .5, 'gravity': 9.81, 'targets': {'joint': .5}}}


class SimulationTests(unittest.TestCase):
    def test_real_physics_targets_and_input_immutability(self):
        payload = fixture()
        before = copy.deepcopy(payload)
        result = simulate(payload)
        self.assertEqual(payload, before)
        self.assertIn('MuJoCo', result['engine'])
        self.assertEqual(len(result['frames']), 11)
        self.assertAlmostEqual(result['frames'][-1]['time'], .5)
        self.assertGreater(result['frames'][-1]['joints']['joint'], .1)
        self.assertLess(result['frames'][-1]['joints']['joint'], 1.01)
        self.assertEqual(result['frames'][-1]['poses']['base']['position'], [0,0,0])
        self.assertAlmostEqual(result['frames'][0]['poses']['arm']['position'][2], .2)

    def test_saved_placement_translates_the_entire_robot(self):
        payload = fixture()
        payload['robot']['placement'] = [.3, -.2, .15]
        result = simulate(payload)
        first = result['frames'][0]['poses']
        self.assertEqual(first['base']['position'], [.3, -.2, .15])
        self.assertAlmostEqual(first['arm']['position'][2], .35)
        self.assertEqual(first['arm']['position'][:2], [.3, -.2])
        self.assertEqual(result['frames'][-1]['poses']['base']['position'], [.3, -.2, .15])

    def test_prismatic_and_continuous_joints(self):
        for kind in ['prismatic', 'continuous']:
            payload = fixture()
            payload['robot']['joints'][0]['type'] = kind
            payload['config']['gravity'] = 0
            result = simulate(payload)
            self.assertGreater(result['frames'][-1]['joints']['joint'], .01)

    def test_missing_inertia_and_out_of_range_targets_rejected(self):
        payload = fixture()
        payload['robot']['links'][1]['inertia'] = [0]*6
        with self.assertRaisesRegex(ValueError, 'inertia'): simulate(payload)
        payload = fixture()
        payload['config']['targets']['joint'] = 3
        with self.assertRaisesRegex(ValueError, 'outside joint limits'): simulate(payload)

    def test_disconnected_graph_rejected(self):
        payload = fixture()
        payload['robot']['joints'] = []
        with self.assertRaisesRegex(ValueError, 'one assembly'): simulate(payload)


if __name__ == '__main__': unittest.main()
