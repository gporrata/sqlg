create table customers (
  id serial primary key,
  name varchar(120) not null,
  email varchar(180) not null
);

create table orders (
  id serial primary key,
  customer_id int not null references customers(id),
  order_number varchar(40) not null,
  total numeric(10, 2) not null
);

create table order_items (
  id serial primary key,
  order_id int not null references orders(id),
  sku varchar(40) not null,
  quantity int not null
);

insert into customers (name, email) values
  ('Ada Lovelace', 'ada@example.com'),
  ('Grace Hopper', 'grace@example.com');

insert into orders (customer_id, order_number, total) values
  (1, 'PG-1001', 199.50),
  (1, 'PG-1002', 49.99),
  (2, 'PG-1003', 75.25);

insert into order_items (order_id, sku, quantity) values
  (1, 'SQL-BOOK', 1),
  (1, 'QUERY-MUG', 2),
  (2, 'INDEX-STICKERS', 3),
  (3, 'PLAN-HOODIE', 1);
