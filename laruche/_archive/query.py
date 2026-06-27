import sqlite3
conn=sqlite3.connect('memoire.db')
c=conn.cursor()
c.execute('SELECT content FROM items WHERE node_id=\"protocols\"')
print(c.fetchall())
