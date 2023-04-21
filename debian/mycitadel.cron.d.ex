#
# Regular cron jobs for the mycitadel package
#
0 4	* * *	root	[ -x /usr/bin/mycitadel_maintenance ] && /usr/bin/mycitadel_maintenance
