# Fish completions for nodaysidle-browser.
#
# The browser accepts URLs or local files (resolved like the address bar), so
# fish's default file completion stays available.

# Common schemes and pages, offered in addition to the file completion.
complete -c nodaysidle-browser -a 'about:blank' -d 'Blank page'
complete -c nodaysidle-browser -a 'https://' -d 'HTTPS URL'
complete -c nodaysidle-browser -a 'http://' -d 'HTTP URL'
complete -c nodaysidle-browser -a 'file://' -d 'Local file URL'
