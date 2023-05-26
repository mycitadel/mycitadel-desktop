!/bin/env bash

dpkg-buildpackage -rfakeroot -us -uc -b
rm -rf debian/.debhelper
rm -rf debian/mycitadel
rm -rf obj-*
