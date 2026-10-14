# Fish completions for nodaysidle-browser.
#
# The browser accepts a URL (resolved like the address bar) or a local file, so
# fish's default file completion stays available and option completions are
# added on top of it.

complete -c nodaysidle-browser -s n -l new-window -d 'Open a new browser window'

# Common schemes and pages, offered in addition to the file completion.
complete -c nodaysidle-browser -a 'about:blank' -d 'Blank page'
complete -c nodaysidle-browser -a 'https://' -d 'HTTPS URL'
complete -c nodaysidle-browser -a 'http://' -d 'HTTP URL'
complete -c nodaysidle-browser -a 'file://' -d 'Local file URL'
