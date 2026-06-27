import sqlite3; conn = sqlite3.connect('memoire.db'); c = conn.cursor(); c.execute("SELECT * FROM items WHERE node_id='capacities.skills.web_research'"); print(c.fetchall())
