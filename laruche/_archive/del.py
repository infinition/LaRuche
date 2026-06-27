import sqlite3
conn=sqlite3.connect('memoire.db')
c=conn.cursor()
c.execute('DELETE FROM items WHERE node_id=\"protocols\"')
c.execute('DELETE FROM nodes WHERE id=\"protocols\"')
conn.commit()
print(f'Deleted items: {c.rowcount}')
