"""The manual's prose for the manifest fields W1-O3 adds (authorship, navigation, level-zero marking).
Apply after W1-O3 lands; the fact ids are the ones the packet prescribes — verify with --citations."""
import io
import os
import sys

sys.stdout.reconfigure(encoding="utf-8")
root = os.getcwd()
EN = os.path.join(root, "vibevm", "vibepacks", "org.vibevm.core", "vibevm-docs", "v0.1.0", "vibevm", "vibespecs")
RU = os.path.join(root, "vibevm", "vibepacks", "org.vibevm.core", "vibevm-docs-ru", "v1.0.0", "vibevm", "vibespecs")
# the packages may have moved to v1.0.0 by the time this runs
for base in (EN,):
    if not os.path.isdir(base):
        EN = base.replace("v0.1.0", "v1.0.0")

R_AUTH = '<rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#CARD-AUTHORSHIP"/>'
R_NAV = '<rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#NAV-PINNED"/>'
R_L0M = '<rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#LEVEL-ZERO-MARKED"/>'


def swap(p, old, new):
    s = io.open(p, encoding="utf-8").read()
    n = s.count(old)
    if n != 1:
        print("NOT FOUND (%d):" % n, p[len(root):], old[:70])
        return
    io.open(p, "w", encoding="utf-8", newline="\n").write(s.replace(old, new))
    print("patched:", p[len(root):])


# --- authoring/write-documentation.xml ---
swap(os.path.join(EN, "authoring", "write-documentation.xml"),
     '    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#COMPANION-NAME"/>\n  </the-manifest>',
     '    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#COMPANION-NAME"/>\n'
     '    <p>Two more declarations are for the reader rather than the site. `authorship` in `[package]` says who wrote the prose, `human`, `ai` or `mixed`; it is a fact about the document a shelf can filter by, never an attribution of the commits, whose law is the repository\'s own. `[navigation]` pins the pages a newcomer should see first and names the sections of the page tree; every other page keeps the manifest\'s order.</p>\n'
     '    ' + R_AUTH + '\n    ' + R_NAV + '\n  </the-manifest>')
swap(os.path.join(RU, "authoring", "write-documentation.xml"),
     '    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#COMPANION-NAME"/>\n  </the-manifest>',
     '    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#COMPANION-NAME"/>\n'
     '    <p>Ещё два объявления адресованы читателю, а не сайту. `authorship` в `[package]` говорит, кто написал прозу: `human`, `ai` или `mixed`; это факт о документе, по которому полка умеет фильтровать, и никогда не атрибуция коммитов, закон которой у репозитория свой. `[navigation]` припиняет страницы, которые новичок должен увидеть первыми, и называет разделы дерева страниц; остальные страницы сохраняют порядок манифеста.</p>\n'
     '    ' + R_AUTH + '\n    ' + R_NAV + '\n  </the-manifest>')
swap(os.path.join(EN, "authoring", "write-documentation.xml"),
     '    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#LEVEL-ZERO"/>\n  </edge-cases>',
     '    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#LEVEL-ZERO"/>\n'
     '    <p>Such a rendering says so in its manifest, so a shelf marks it as generated, and the page of a bridge keeps the maintainer of the bridge apart from the author of what it wraps.</p>\n'
     '    ' + R_L0M + '\n  </edge-cases>')
swap(os.path.join(RU, "authoring", "write-documentation.xml"),
     '    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#LEVEL-ZERO"/>\n  </edge-cases>',
     '    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#LEVEL-ZERO"/>\n'
     '    <p>Такая отрисовка говорит об этом в своём манифесте, так что полка помечает её как сгенерированную, а страница моста держит сопровождающего моста отдельно от автора того, что он оборачивает.</p>\n'
     '    ' + R_L0M + '\n  </edge-cases>')

# --- reference/manifest.xml ---
swap(os.path.join(EN, "reference", "manifest.xml"),
     "      <tr><td>`describes`</td>",
     "      <tr><td>`authorship`</td><td>who wrote the prose of a `doc` package: `human`, `ai` or `mixed`; a reader's filter, not the commits' attribution</td></tr>\n"
     "      <tr><td>`describes`</td>")
swap(os.path.join(EN, "reference", "manifest.xml"),
     '    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#CARD-FIELDS"/>\n    <p>`[project]`',
     '    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#CARD-FIELDS"/>\n    ' + R_AUTH + '\n    <p>`[project]`')
swap(os.path.join(EN, "reference", "manifest.xml"),
     "      <tr><td>`[media]`</td>",
     "      <tr><td>`[navigation]`</td><td>`pinned`, the document paths listed first, and `[[navigation.section]]` rows with `id` and `title` for the folders of the page tree</td><td>`doc`</td></tr>\n"
     "      <tr><td>`[media]`</td>")
swap(os.path.join(EN, "reference", "manifest.xml"),
     '    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE"/>\n  </documentation-tables>',
     '    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE"/>\n    ' + R_NAV + '\n  </documentation-tables>')
swap(os.path.join(RU, "reference", "manifest.xml"),
     "      <tr><td>`describes`</td>",
     "      <tr><td>`authorship`</td><td>кто написал прозу пакета `doc`: `human`, `ai` или `mixed`; фильтр читателя, а не атрибуция коммитов</td></tr>\n"
     "      <tr><td>`describes`</td>")
swap(os.path.join(RU, "reference", "manifest.xml"),
     '    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#CARD-FIELDS"/>\n    <p>`[project]`',
     '    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#CARD-FIELDS"/>\n    ' + R_AUTH + '\n    <p>`[project]`')
swap(os.path.join(RU, "reference", "manifest.xml"),
     "      <tr><td>`[media]`</td>",
     "      <tr><td>`[navigation]`</td><td>`pinned`, пути документов, которые перечисляются первыми, и строки `[[navigation.section]]` с `id` и `title` для папок дерева страниц</td><td>`doc`</td></tr>\n"
     "      <tr><td>`[media]`</td>")
swap(os.path.join(RU, "reference", "manifest.xml"),
     '    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE"/>\n  </documentation-tables>',
     '    <rule ref="spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE"/>\n    ' + R_NAV + '\n  </documentation-tables>')
