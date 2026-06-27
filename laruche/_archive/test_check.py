import sqlite3; conn = sqlite3.connect('memoire.db'); c = conn.cursor(); c.execute("SELECT COUNT(*) FROM items WHERE node_id='capacities.skills.web-research'"); print(c.fetchall())
