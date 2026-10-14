"""Validate inert design data; never import or execute payload files."""
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parent
DOC = ROOT.parents[2] / 'vibevm/vibespecs/common/PROP-074-local-derivatives.xml'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def inventory(directory):
    rows = []
    for path in sorted(directory.rglob('*')):
        assert not path.is_symlink(), path
        if path.is_file():
            data = path.read_bytes()
            rows.append({'path': path.relative_to(directory).as_posix(),
                         'sha256': digest(data), 'bytes': len(data)})
    return rows


def validate():
    manifest = json.loads((ROOT / 'manifest.json').read_text())
    doc = ET.parse(DOC).getroot()
    anchors = [node.attrib['id'] for node in doc.iter() if 'id' in node.attrib]
    assert len(set(anchors)) == len(anchors)
    assert not any(node.attrib.get('fact') == 'true' for node in doc.iter())
    cases = manifest['cases']
    assert [case['id'] for case in cases] == [
        'commonmark', 'xml-data', 'vibe-spec', 'opaque-file', 'nested-tree']
    count = xml_count = 0
    for case in cases:
        assert case['anchor'] in anchors
        base, local, expected = case['base'], case['local'], case['expected']
        assert base['immutable'] and base['sealed'] and case['demand_evidence'] is None
        assert base['coordinate'] != local['coordinate']
        assert case['compatibility']['target'] == base['coordinate']
        assert expected['eligible_for_separate_derivative']
        assert expected['base_unchanged'] and expected['local_survives_output_deletion']
        assert case['activation']['state'] == 'not-requested'
        assert not case['activation']['inherits_upstream_grants']
        assert not case['activation']['executes_code']
        assert len(case['later_executable_requirements']) == 5
        assert case['missing_behavior_and_owner'] and case['existing_mechanism']
        roots = [base['capture_root'], local['authored_root'], expected['root']]
        assert len(set(roots)) == 3
        for label, value, root in zip(['base', 'local', 'expected'], [base, local, expected], roots):
            assert root == case['id'] + '/' + label
            actual = inventory(ROOT / root)
            assert actual == value['files'], (case['id'], label)
            count += len(actual)
            for row in actual:
                data = (ROOT / root / row['path']).read_bytes()
                if row['path'].endswith('.xml'):
                    ET.fromstring(data)
                    xml_count += 1
        assert base['files'] != local['files']
        assert local['files'] == expected['files']
        base_hash = digest(json.dumps(base['files'], sort_keys=True, separators=(',', ':')).encode())
        assert base_hash == base['inventory_sha256'] == case['compatibility']['base_inventory_sha256']
    for label in ['base', 'local', 'expected']:
        cm = (ROOT / 'commonmark' / label / 'guide.md').read_bytes()
        assert b'```sh\nprintf DO_NOT_EXECUTE\n```' in cm
        assert cm.endswith(b'## Reference\nKeep this section.\n')
        xml = (ROOT / 'xml-data' / label / 'catalog.xml').read_bytes()
        assert xml.startswith(b'\xef\xbb\xbf') and b'a&amp;b' in xml
        assert b'<!-- preserve spelling -->' in xml and b'<script>printf DO_NOT_EXECUTE</script>' in xml
        opaque = (ROOT / 'opaque-file' / label / 'payload.bin').read_bytes()
        assert opaque.startswith(b'\x00\xff\x80') and opaque.endswith(b'\r\nprintf DO_NOT_EXECUTE\x00')
        script = ROOT / 'nested-tree' / label / 'scripts/helper.sh'
        assert script.read_bytes() == b'#!/bin/sh\nprintf DO_NOT_EXECUTE\n'
        assert not script.stat().st_mode & 0o111
    b = ROOT / 'nested-tree/base'
    l = ROOT / 'nested-tree/local'
    assert (b / 'docs/obsolete.txt').exists() and not (l / 'docs/obsolete.txt').exists()
    assert not (b / 'docs/local.txt').exists() and (l / 'docs/local.txt').exists()
    assert (b / 'docs/keep.txt').read_bytes() == (l / 'docs/keep.txt').read_bytes()
    assert manifest['tree_neighbor']['expected_after_update_and_uninstall'] == 'unchanged'
    print(json.dumps({'families': len(cases), 'payload_files': count,
                      'payload_xml_documents': xml_count, 'canonical_xml_documents': 1,
                      'manifest_json_documents': 1, 'unique_anchors': len(anchors),
                      'future_service_cases_executed': 0, 'result': 'passed'}))


if __name__ == '__main__':
    validate()
