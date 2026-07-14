# Compose {#root}

A host doc that pulls one dependency and splices one section, so the inline
compiler must emit the dependency first and expand the macro.

#use spec://vibevm/common/PROP-000#commits
#embed spec://org.vibevm.demo/demo-lib/contract/API#root
