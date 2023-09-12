 $env:Path="C:\gtk-build\gtk\x64\release\bin;"+$env:Path
 $env:LIB="C:\gtk-build\gtk\x64\release\lib;"+$env:LIB
 $env:INCLUDE="C:\gtk-build\gtk\x64\release\include;C:\gtk-build\gtk\x64\release\include\cairo;C:\gtk-build\gtk\x64\release\include\glib-2.0;C:\gtk-build\gtk\x64\release\include\gobject-introspection-1.0;C:\gtk-build\gtk\x64\release\lib\glib-2.0\include;"+$env:INCLUDE
 cargo build --all-features --release
