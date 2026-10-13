"""Bounded local MuJoCo adapter. Input and output paths are supplied by the host."""
import base64
import json
import math
import sys
import time
import xml.etree.ElementTree as ET


def build_model(payload):
    import mujoco
    import numpy as np

    robot, config = payload['robot'], payload['config']
    links, joints = robot['links'], robot['joints']
    if not 0 < len(links) <= 256:
        raise ValueError('Simulation supports 1–256 links.')
    duration, gravity = float(config['duration']), float(config['gravity'])
    if not math.isfinite(duration) or not 0.1 <= duration <= 10 or not math.isfinite(gravity) or not 0 <= gravity <= 20:
        raise ValueError('Use a duration of 0.1–10 seconds and gravity of 0–20 m/s².')
    placement = robot.get('placement', [0, 0, 0])
    if len(placement) != 3 or not all(math.isfinite(float(v)) for v in placement):
        raise ValueError('Robot placement must contain three finite coordinates.')
    names = {link['id']: 'body_' + str(i) for i, link in enumerate(links)}
    if len(names) != len(links):
        raise ValueError('Duplicate link IDs.')
    incoming = {}
    for j in joints:
        if j['parent'] not in names or j['child'] not in names or j['child'] in incoming:
            raise ValueError('Invalid joint tree.')
        incoming[j['child']] = j
    roots = [l for l in links if l['id'] not in incoming]
    if len(roots) != 1:
        raise ValueError('Connect all links into one assembly.')
    fmt = lambda values: ' '.join(str(float(v)) for v in values)

    def rotation(rpy):
        r, p, y = rpy
        cr, sr, cp, sp, cy, sy = math.cos(r), math.sin(r), math.cos(p), math.sin(p), math.cos(y), math.sin(y)
        return np.array([[cy*cp, cy*sp*sr-sy*cr, cy*sp*cr+sy*sr], [sy*cp, sy*sp*sr+cy*cr, sy*sp*cr-cy*sr], [-sp, cp*sr, cp*cr]])

    def quat(rpy):
        q = np.zeros(4)
        mujoco.mju_mat2Quat(q, rotation(rpy).flatten())
        return fmt(q)

    xml = ET.Element('mujoco', model='Humaan saved embodiment')
    ET.SubElement(xml, 'compiler', angle='radian', inertiafromgeom='false', fusestatic='false')
    ET.SubElement(xml, 'option', timestep='0.002', gravity=f'0 0 {-gravity}', integrator='implicitfast')
    asset = ET.SubElement(xml, 'asset')
    world = ET.SubElement(xml, 'worldbody')
    ET.SubElement(world, 'geom', name='floor', type='plane', size='5 5 0.1', pos='0 0 0', friction='0.8 0.005 0.0001')
    actuators = ET.SubElement(xml, 'actuator')
    files, controlled, seen = {}, [], set()
    warnings = ['Fixed root at the saved robot placement; floor at 0 m. Contact friction is 0.8. Sensors are not simulated.', 'Position servos use fixed gains (kp=20, kv=2) and joint effort limits. Hardware velocity limits are reported, not enforced.']

    def add(link, parent):
        lid = link['id']
        if lid in seen:
            raise ValueError('Cyclic joint tree.')
        seen.add(lid)
        j = incoming.get(lid)
        origin = j['origin'] if j else {'xyz': placement, 'rpy': [0, 0, 0]}
        body = ET.SubElement(parent, 'body', name=names[lid], pos=fmt(origin['xyz']), quat=quat(origin['rpy']))
        if j:
            mass = float(link['mass'])
            a,b,c,d,e,f = link['inertia']
            inertia = np.array([[a,b,c],[b,d,e],[c,e,f]], dtype=float)
            eigen = np.linalg.eigvalsh(inertia)
            if not math.isfinite(mass) or mass <= 0 or not np.all(np.isfinite(eigen)) or eigen[0] <= 0 or eigen[2] > eigen[0]+eigen[1]+max(eigen)*1e-8:
                raise ValueError(f"{link['name']}: positive mass and physically valid inertia are required.")
            r = rotation(link['inertial']['rpy'])
            tensor = r @ inertia @ r.T
            ET.SubElement(body, 'inertial', pos=fmt(link['inertial']['xyz']), mass=str(mass), fullinertia=fmt([tensor[0,0],tensor[1,1],tensor[2,2],tensor[0,1],tensor[0,2],tensor[1,2]]))
            kind = j['type']
            if kind not in ('fixed', 'revolute', 'continuous', 'prismatic'):
                raise ValueError('Unsupported joint type: ' + kind)
            if kind != 'fixed':
                axis = np.array(j['axis'], dtype=float)
                if not np.all(np.isfinite(axis)) or np.linalg.norm(axis) < 1e-9:
                    raise ValueError('Invalid joint axis.')
                lower, upper, effort, velocity = map(float, (j['lower'],j['upper'],j['effort'],j['velocity']))
                if not all(map(math.isfinite, [lower,upper,effort,velocity])) or (kind != 'continuous' and lower >= upper) or effort <= 0 or velocity <= 0:
                    raise ValueError('Invalid joint limits: ' + j['name'])
                jid = 'joint_' + str(len(controlled))
                attrs = dict(name=jid, type='slide' if kind == 'prismatic' else 'hinge', axis=fmt(axis), damping='0.05', limited='false' if kind == 'continuous' else 'true')
                if kind != 'continuous': attrs['range'] = fmt([lower,upper])
                ET.SubElement(body, 'joint', **attrs)
                ET.SubElement(actuators, 'position', joint=jid, kp='20', kv='2', forcelimited='true', forcerange=fmt([-effort,effort]))
                target = float(config.get('targets', {}).get(j['id'], 0))
                if not math.isfinite(target) or (kind != 'continuous' and not lower <= target <= upper):
                    raise ValueError('Target is outside joint limits: ' + j['name'])
                controlled.append((j, jid, target))
        if not link['collisions']: warnings.append(link['name'] + ': no collision geometry.')
        for shape in link['collisions']:
            size = shape['size']
            attrs = dict(pos=fmt(shape['origin']['xyz']), quat=quat(shape['origin']['rpy']), friction='0.8 0.005 0.0001')
            kind = shape['kind']
            dimensions = size if kind in ('box', 'mesh') else size[:1] if kind == 'sphere' else size[:2]
            if any(not math.isfinite(float(v)) or v <= 0 for v in dimensions): raise ValueError('Invalid collision dimensions.')
            if kind == 'mesh':
                mesh = robot['assets'][shape['asset']]
                if mesh['format'] not in ('stl','obj'): raise ValueError('Unsupported mesh format.')
                mesh_id = 'mesh_' + str(len(files))
                filename = mesh_id + '.' + mesh['format']
                files[filename] = base64.b64decode(mesh['data'], validate=True)
                ET.SubElement(asset, 'mesh', name=mesh_id, file=filename, scale=fmt(size))
                attrs.update(type='mesh', mesh=mesh_id)
                warnings.append(link['name'] + ': mesh collisions use a convex hull.')
            elif kind == 'box': attrs.update(type='box', size=fmt([v/2 for v in size]))
            elif kind == 'sphere': attrs.update(type='sphere', size=fmt(size[:1]))
            elif kind == 'cylinder': attrs.update(type='cylinder', size=fmt([size[0],size[1]/2]))
            else: raise ValueError('Unsupported collision geometry.')
            ET.SubElement(body, 'geom', **attrs)
        for child in links:
            if incoming.get(child['id'], {}).get('parent') == lid: add(child, body)

    add(roots[0], world)
    if len(seen) != len(links): raise ValueError('Disconnected or cyclic links.')
    xml_text = ET.tostring(xml, encoding='unicode')
    model = mujoco.MjModel.from_xml_string(xml_text, files)
    return model, controlled, names, xml_text, warnings


def simulate(payload):
    import mujoco
    import numpy as np

    model, controlled, names, xml_text, warnings = build_model(payload)
    duration = float(payload['config']['duration'])
    data = mujoco.MjData(model)
    for index, (j, name, target) in enumerate(controlled):
        address = model.jnt_qposadr[model.joint(name).id]
        data.qpos[address] = 0 if j['type'] == 'continuous' else max(j['lower'],min(j['upper'],0))
        data.ctrl[index] = target
    mujoco.mj_forward(model, data)
    body_ids = {lid: model.body(name).id for lid, name in names.items()}
    addresses = {j['id']: model.jnt_qposadr[model.joint(name).id] for j,name,_ in controlled}
    frames, max_contacts = [], 0

    def capture():
        frames.append({'time': float(data.time), 'poses': {lid: {'position': data.xpos[bid].tolist(), 'quaternion': [*data.xquat[bid][1:].tolist(),float(data.xquat[bid][0])]} for lid,bid in body_ids.items()}, 'joints': {jid: float(data.qpos[address]) for jid,address in addresses.items()}})

    capture()
    start = time.monotonic()
    steps = round(duration/model.opt.timestep)
    for step in range(steps):
        mujoco.mj_step(model, data)
        max_contacts = max(max_contacts, int(data.ncon))
        if not np.all(np.isfinite(data.qpos)) or any(w.number for w in data.warning):
            raise ValueError('Simulation became unstable. Review mass, inertia, collisions, and joint settings.')
        if time.monotonic()-start > 45: raise TimeoutError('Simulation exceeded the 45-second execution limit.')
        if (step+1) % 25 == 0 or step+1 == steps:
            mujoco.mj_forward(model, data)
            capture()
    return {'engine': 'MuJoCo ' + mujoco.__version__, 'pythonVersion': sys.version, 'adapterVersion': 2, 'floorHeight': 0, 'timestep': float(model.opt.timestep), 'frames': frames, 'warnings': list(dict.fromkeys(warnings)), 'maxContacts': max_contacts, 'modelXml': xml_text}


if __name__ == '__main__':
    try:
        with open(sys.argv[1]) as f: result = simulate(json.load(f))
    except ImportError:
        result = {'error': 'MuJoCo is not installed in this Python environment. Install mujoco and select that environment’s Python executable.'}
    except Exception as error:
        result = {'error': str(error)}
    with open(sys.argv[2], 'w') as f: json.dump(result, f, allow_nan=False)
