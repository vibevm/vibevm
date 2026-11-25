# `doc:vibevm-docs-ru` — руководство VibeVM по-русски {#root}

<status stage="doc" state="work" audience="user"/>

@fact:PACKAGE-PURPOSE Этот пакет — русская адаптация руководства VibeVM
`org.vibevm.core/vibevm-docs`: те же страницы, те же якоря, те же примеры
по ссылке, но каждое предложение написано по-русски заново, а не
переведено. Он документирует координату хоста `org.vibevm.core/vibevm` и
читается, а не устанавливается. @status:doc/work

@fact:HOW-TO-READ Читать на сайте по адресу `https://vibevm.org/doc/`, выбрав
русский язык документации, или у себя на машине: `vibe cache add
org.vibevm.core/vibevm-docs-ru` прогревает адаптацию, `vibe doc serve`
показывает её. @status:doc/work

@fact:HOW-IT-IS-KEPT Адаптация зеркалит источник блок в блок и проверяется
`vibe doc check --translations`; нормативные правила остаются на языке
спецификации, а примеры команд проверяются один раз, на источнике
(spec://org.vibevm.core/vibevm/common/PROP-057#LOC-MIRROR). @status:doc/work
