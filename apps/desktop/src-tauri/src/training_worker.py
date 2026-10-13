"""Humaan joint-reach trainer v1: bounded CEM search of feedback controller gains.

The host prepends simulation_worker.py with its CLI entrypoint removed. This same
source runs locally and on an existing SSH host. No project paths are required.
"""

def train(experiment):
    import hashlib
    import mujoco
    import numpy as np

    if mujoco.__version__ != '3.2.7':
        raise ValueError('Training adapter v1 requires MuJoCo 3.2.7. Select the managed runtime or install mujoco==3.2.7 on your cloud host.')
    started = time.monotonic()
    env, task, config = experiment['environment'], experiment['task'], experiment['config']
    robot = experiment['robot']
    targets = {j['id']: (j['lower']+j['upper'])/2 for j in robot['joints'] if j['type'] != 'fixed'}
    model, controlled, names, xml_text, warnings = build_model({'robot': robot, 'config': {'duration': env['duration'], 'gravity': env['gravity'], 'targets': targets}})
    if not controlled or len(controlled) > 32:
        raise ValueError('Training needs 1–32 actuated joints.')
    if any(j['type'] == 'continuous' for j, _, _ in controlled):
        raise ValueError('Joint-reach training requires finite limits; use revolute or prismatic joints.')
    lower = np.array([j['lower'] for j, _, _ in controlled])
    span = np.array([j['upper']-j['lower'] for j, _, _ in controlled])
    goal = lower + task['targetFraction']*span
    qadr = np.array([model.jnt_qposadr[model.joint(name).id] for _, name, _ in controlled])
    dadr = np.array([model.jnt_dofadr[model.joint(name).id] for _, name, _ in controlled])
    steps = round(env['duration']/model.opt.timestep)
    eval_seeds = [1000000+i for i in range(5)]
    # All candidates see the same starts; evaluation starts are held out.
    train_seeds = [config['seed']*10+i for i in range(3)]
    rng = np.random.default_rng(config['seed'])

    def rollout(weights, seed, capture=False):
        data = mujoco.MjData(model)
        initial = np.random.default_rng(seed).uniform(0.25, 0.75, len(controlled))
        data.qpos[qadr] = lower + initial*span
        mujoco.mj_forward(model, data)
        frames, errors = [], []
        def frame():
            mujoco.mj_forward(model, data)
            frames.append({'time': float(data.time), 'poses': {lid: {'position': data.xpos[model.body(name).id].tolist(), 'quaternion': [*data.xquat[model.body(name).id][1:].tolist(), float(data.xquat[model.body(name).id][0])]} for lid, name in names.items()}, 'joints': {j['id']: float(data.qpos[qadr[i]]) for i, (j, _, _) in enumerate(controlled)}})
        if capture: frame()
        for step in range(steps):
            # Observations: joint positions and velocities. Actions: position servo targets.
            data.ctrl[:] = np.clip(data.qpos[qadr] + weights[:len(controlled)]*(goal-data.qpos[qadr]) - weights[len(controlled):]*data.qvel[dadr], lower, lower+span)
            mujoco.mj_step(model, data)
            if any(w.number for w in data.warning) or not np.all(np.isfinite(data.qpos)):
                raise ValueError('Training simulation became unstable; review the robot physics.')
            errors.append(float(np.mean(np.abs((data.qpos[qadr]-goal)/span))))
            if step % 25 == 0 and time.monotonic()-started > 180:
                raise TimeoutError('Training exceeded the 180-second worker limit. Reduce iterations or episode duration.')
            if capture and ((step+1) % 25 == 0 or step+1 == steps): frame()
        # Success requires every joint to be within tolerance at episode end.
        final_error = float(np.max(np.abs((data.qpos[qadr]-goal)/span)))
        return {'loss': float(np.mean(errors)), 'finalError': final_error, 'success': final_error <= task['tolerance'], 'frames': frames}

    def score(weights):
        return float(np.mean([rollout(weights, seed)['loss'] for seed in train_seeds]))

    n = len(controlled)
    best = np.r_[np.ones(n), np.zeros(n)]
    best_loss = score(best)
    baseline = best.copy()
    mean, std = best.copy(), np.r_[np.ones(n)*0.5, np.ones(n)*0.1]
    history = [{'iteration': 0, 'loss': best_loss}]
    for iteration in range(config['iterations']):
        population = np.clip(rng.normal(mean, std, (config['population'], n*2)), np.zeros(n*2), np.r_[np.ones(n)*3, np.ones(n)*0.5])
        population[0] = best
        scores = np.array([score(candidate) for candidate in population])
        elite = population[np.argsort(scores)[:max(2, config['population']//4)]]
        index = int(np.argmin(scores))
        if scores[index] < best_loss: best, best_loss = population[index].copy(), float(scores[index])
        mean, std = elite.mean(axis=0), np.maximum(elite.std(axis=0), 0.01)
        history.append({'iteration': iteration+1, 'loss': best_loss})
        print('HUMAAN_PROGRESS:' + json.dumps(history[-1]), flush=True)
    def evaluate(weights):
        episodes = [rollout(weights, seed) for seed in eval_seeds]
        return {'successRate': sum(e['success'] for e in episodes)/len(episodes), 'meanError': float(np.mean([e['finalError'] for e in episodes])), 'meanLoss': float(np.mean([e['loss'] for e in episodes])), 'seeds': eval_seeds, 'episodes': [{k:v for k,v in e.items() if k != 'frames'} for e in episodes]}
    protocol = {'adapter': 1, 'engine': mujoco.__version__, 'environment': env, 'task': task, 'evaluationSeeds': eval_seeds}
    # Names are labels, not part of the numerical comparison protocol.
    protocol['environment'] = {k:v for k,v in env.items() if k not in ('id','name','createdAt')}
    protocol['task'] = {k:v for k,v in task.items() if k not in ('id','name','createdAt')}
    comparison_key = hashlib.sha256(json.dumps(protocol, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    return {'engine': 'MuJoCo '+mujoco.__version__, 'adapterVersion': 1, 'pythonVersion': sys.version, 'history': history, 'evaluation': evaluate(best), 'baseline': evaluate(baseline), 'comparisonKey': comparison_key,
        'policy': {'format': 'humaan-joint-feedback-v1', 'jointIds': [j['id'] for j,_,_ in controlled], 'observations': ['joint_position', 'joint_velocity'], 'actions': 'position_servo_target', 'gain': best[:n].tolist(), 'damping': best[n:].tolist(), 'target': goal.tolist(), 'lower': lower.tolist(), 'upper': (lower+span).tolist()},
        'frames': rollout(best, eval_seeds[0], True)['frames'], 'modelXml': xml_text, 'warnings': list(dict.fromkeys(warnings)), 'elapsedSeconds': time.monotonic()-started}


def training_main():
    try:
        experiment = json.load(sys.stdin)
        if len(experiment.get('robot', {}).get('links', [])) > 256: raise ValueError('Too many links.')
        result = train(experiment)
        result['experimentId'] = experiment['id']
    except Exception as error:
        result = {'error': str(error)}
    print('\nHUMAAN_RESULT:' + json.dumps(result, allow_nan=False), flush=True)

if __name__ == '__main__':
    training_main()
