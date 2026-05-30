create table customers (
  id int primary key auto_increment,
  name varchar(120) not null,
  email varchar(180) not null
);

create table orders (
  id int primary key auto_increment,
  customer_id int not null,
  order_number varchar(40) not null,
  total decimal(10, 2) not null,
  constraint fk_orders_customer foreign key (customer_id) references customers(id)
);

create table order_items (
  id int primary key auto_increment,
  order_id int not null,
  sku varchar(40) not null,
  quantity int not null,
  constraint fk_items_order foreign key (order_id) references orders(id)
);

insert into customers (name, email) values
  ('Ada Lovelace', 'ada@example.com'),
  ('Grace Hopper', 'grace@example.com');

insert into orders (customer_id, order_number, total) values
  (1, 'MYSQL-1001', 199.50),
  (1, 'MYSQL-1002', 49.99),
  (2, 'MYSQL-1003', 75.25);

insert into order_items (order_id, sku, quantity) values
  (1, 'SQL-BOOK', 1),
  (1, 'QUERY-MUG', 2),
  (2, 'INDEX-STICKERS', 3),
  (3, 'PLAN-HOODIE', 1);
