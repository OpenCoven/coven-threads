"""Validate the bounded Nextest JUnit dialect and preserve retry observations."""
import datetime
import json
import re
import sys
import xml.etree.ElementTree as ET


def parse(data):
    if not data or b'\0' in data or len(data) > 64 * 1024 * 1024 or re.search(br"<!\s*(DOCTYPE|ENTITY)", data, re.I):
        raise ValueError("unsupported XML input")
    root = ET.fromstring(data)
    if root.tag != 'testsuites' or not re.fullmatch(r'[a-fA-F0-9]{8}(?:-[a-fA-F0-9]{4}){3}-[a-fA-F0-9]{12}', root.get('uuid', '')):
        raise ValueError('Nextest report identity missing')
    timestamp = datetime.datetime.fromisoformat(root.attrib['timestamp'].replace('Z', '+00:00'))
    if timestamp.tzinfo is None:
        raise ValueError('report timestamp lacks timezone')
    if len(list(root.iter())) > 200000:
        raise ValueError('report node bound exceeded')
    counts = dict.fromkeys(['tests', 'passed', 'failures', 'errors', 'skipped', 'flaky', 'retried', 'retries', 'firstAttemptPassed'], 0)
    cases, identities, suites = [], set(), set()

    def counter(node, key):
        value = node.get(key, '0' if key == 'skipped' else '')
        if not re.fullmatch(r'0|[1-9][0-9]*', value) or int(value) > 9007199254740991:
            raise ValueError('invalid summary counter')
        return int(value)

    def verify(node, actual):
        for key in ['tests', 'failures', 'errors', 'skipped']:
            if counter(node, key) != actual[key]:
                raise ValueError('summary contradicts testcase outcomes')

    for suite in root:
        name = suite.get('name')
        if suite.tag != 'testsuite' or not name or name in suites:
            raise ValueError('ambiguous or unsupported testsuite')
        suites.add(name)
        local = dict.fromkeys(counts, 0)
        for case in suite:
            if case.tag in ['system-out', 'system-err', 'properties']:
                if case.tag == 'properties':
                    if any(p.tag != 'property' or len(p) for p in case):
                        raise ValueError('unsupported suite properties')
                elif len(case):
                    raise ValueError('nested suite output')
                continue
            identity = (name, case.get('classname'), case.get('name'))
            if case.tag != 'testcase' or not all(identity) or identity in identities:
                raise ValueError('invalid or duplicate testcase')
            identities.add(identity)
            tags = [child.tag for child in case]
            allowed = ['failure', 'error', 'skipped', 'flakyFailure', 'flakyError', 'rerunFailure', 'rerunError', 'system-out', 'system-err', 'properties']
            if any(tag not in allowed for tag in tags):
                raise ValueError('unsupported testcase outcome')
            terminals = [tag for tag in tags if tag in ['failure', 'error', 'skipped']]
            if len(terminals) > 1:
                raise ValueError('contradictory terminal outcomes')
            flaky = sum(tags.count(t) for t in ['flakyFailure', 'flakyError'])
            reruns = sum(tags.count(t) for t in ['rerunFailure', 'rerunError'])
            if (flaky and reruns) or (reruns and terminals not in [['failure'], ['error']]) or (terminals == ['skipped'] and flaky):
                raise ValueError('contradictory retry outcomes')
            for child in case:
                if child.tag == 'properties' and any(p.tag != 'property' or len(p) for p in child):
                    raise ValueError('unsupported testcase properties')
                if child.tag != 'properties' and any(c.tag not in ['system-out', 'system-err'] or len(c) for c in child):
                    raise ValueError('unsupported nested testcase data')
            outcome = terminals[0] if terminals else 'passed'
            row = dict(suite=name, classname=identity[1], name=identity[2], outcome=outcome,
                       flakyFailures=tags.count('flakyFailure'), flakyErrors=tags.count('flakyError'),
                       rerunFailures=tags.count('rerunFailure'), rerunErrors=tags.count('rerunError'))
            cases.append(row)
            local['tests'] += 1
            local[dict(failure='failures', error='errors', skipped='skipped', passed='passed')[outcome]] += 1
            local['flaky'] += bool(flaky)
            local['retried'] += bool(flaky or reruns)
            local['retries'] += flaky + reruns
            local['firstAttemptPassed'] += outcome == 'passed' and not (flaky or reruns)
        verify(suite, local)
        for key, value in local.items():
            counts[key] += value
    verify(root, counts)
    if not counts['tests']:
        raise ValueError('zero tests cannot qualify')
    return dict(uuid=root.attrib['uuid'], timestamp=root.attrib['timestamp'], counts=counts,
                cases=sorted(cases, key=lambda c: (c['suite'], c['classname'], c['name'])))


if __name__ == '__main__':
    try:
        print(json.dumps(parse(sys.stdin.buffer.read(64 * 1024 * 1024 + 1)), sort_keys=True))
    except (ValueError, KeyError, ET.ParseError, OverflowError):
        print('Nextest JUnit refused', file=sys.stderr)
        sys.exit(1)
