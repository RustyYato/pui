# v 0.5.2

* Updated feature flags documentation
* Added README
* removed `pui::core::ty`
    * it's too easy to create an unsound `Type` without runtime checks
    * These runtime checks are alreadyperformed by `pui::core::dynamic`

# v 0.5.1 - Initial Release
