-- === Schema ===

begin
    execute immediate 'drop user schema3 cascade';
    execute immediate 'drop user schema4 cascade';
exception when others then
    IF SQLCODE != -1918 THEN
        RAISE;
    END IF;
end;



CREATE USER schema3 IDENTIFIED BY schema1_password;
GRANT CONNECT, RESOURCE, CREATE SEQUENCE, CREATE PROCEDURE, CREATE TRIGGER TO schema1;

CREATE USER schema4 IDENTIFIED BY schema2_password;
GRANT CONNECT, RESOURCE, CREATE SEQUENCE, CREATE PROCEDURE, CREATE TRIGGER TO schema2;

-- === Tables (10) ===
CREATE TABLE schema3.emp (id NUMBER PRIMARY KEY, start_date DATE DEFAULT SYSDATE); -- Changed column
CREATE TABLE schema3.dept (dept_id NUMBER PRIMARY KEY, location VARCHAR2(100)); -- Added column
CREATE TABLE schema3.projects (project_id NUMBER PRIMARY KEY, project_name VARCHAR2(100) UNIQUE);
CREATE TABLE schema3.clients (client_id NUMBER PRIMARY KEY, client_type CHAR(1) CHECK (client_type IN ('A', 'B')));
CREATE TABLE schema3.locations (loc_id NUMBER PRIMARY KEY, address VARCHAR2(255));
CREATE TABLE schema4.emp (id NUMBER PRIMARY KEY, hire_date DATE); -- Different column name
CREATE TABLE schema4.salary (salary_id NUMBER PRIMARY KEY, pay_grade CHAR(2) NOT NULL);
CREATE TABLE schema4.inventory (item_id NUMBER PRIMARY KEY, quantity NUMBER DEFAULT 0);
CREATE TABLE schema4.customers (cust_id NUMBER PRIMARY KEY, email VARCHAR2(100));
CREATE TABLE schema4.transactions (txn_id NUMBER PRIMARY KEY, txn_date TIMESTAMP);

-- === Indexes (3) ===
CREATE INDEX schema4.idx_cust_email ON schema4.customers (email);
CREATE UNIQUE INDEX schema3.u_idx_loc_addr ON schema3.locations (address);

-- === Sequences (3) ===
CREATE SEQUENCE schema3.emp_seq START WITH 100 INCREMENT BY 1;
CREATE SEQUENCE schema4.salary_seq START WITH 500 INCREMENT BY 1;
CREATE SEQUENCE schema3.client_seq START WITH 1; -- New sequence

-- === Views (5) ===
CREATE OR REPLACE VIEW schema3.v_emp AS
SELECT id, start_date FROM schema3.emp; -- Updated view
CREATE OR REPLACE VIEW schema3.v_active_projects AS
SELECT project_id, project_name FROM schema3.projects WHERE project_id < 50; -- New view
CREATE OR REPLACE VIEW schema4.v_salary AS
SELECT salary_id, pay_grade FROM schema4.salary; -- Updated view
CREATE OR REPLACE VIEW schema4.v_item_count AS
SELECT item_id, quantity FROM schema4.inventory WHERE quantity > 0; -- New view
CREATE OR REPLACE VIEW schema3.v_client_a AS
SELECT client_id FROM schema3.clients WHERE client_type = 'A'; -- New view

-- === Types (4) ===
CREATE OR REPLACE TYPE schema3.address_t AS OBJECT (
                                                       street VARCHAR2(50),
                                                       city   VARCHAR2(50)
                                                   );
/
CREATE OR REPLACE TYPE schema3.phone_t AS OBJECT ( -- New type
                                                     area_code VARCHAR2(5),
                                                     number    VARCHAR2(15)
                                                 );
/
CREATE OR REPLACE TYPE schema4.department_t AS OBJECT (
                                                          dept_name VARCHAR2(50)
                                                      );
/
CREATE OR REPLACE TYPE schema4.product_t AS OBJECT ( -- New type
                                                       product_name VARCHAR2(100),
                                                       price        NUMBER(10, 2)
                                                   );
/

-- === Procedures (4) ===
CREATE OR REPLACE PROCEDURE schema3.add_emp(p_id NUMBER) AS -- Different signature
BEGIN
    INSERT INTO schema3.emp(id) VALUES (p_id);
END;
/
CREATE OR REPLACE PROCEDURE schema3.log_location(p_loc_id NUMBER) AS -- New procedure
BEGIN
    DBMS_OUTPUT.PUT_LINE('Location ID logged: ' || p_loc_id);
END;
/
CREATE OR REPLACE PROCEDURE schema4.add_salary(p_salary_id NUMBER) AS -- Different signature
BEGIN
    INSERT INTO schema4.salary(salary_id) VALUES (p_salary_id);
END;
/
CREATE OR REPLACE PROCEDURE schema4.process_txn(p_txn_id NUMBER) AS -- New procedure
BEGIN
    UPDATE schema4.transactions SET txn_date = SYSDATE WHERE txn_id = p_txn_id;
END;
/

-- === Functions (4) ===
CREATE OR REPLACE FUNCTION schema3.get_client_count RETURN NUMBER AS -- New function
    v_count NUMBER;
BEGIN
    SELECT COUNT(*) INTO v_count FROM schema3.clients;
    RETURN v_count;
END;
/
CREATE OR REPLACE FUNCTION schema3.get_days_worked(p_id NUMBER) RETURN NUMBER AS -- New function
    v_start_date DATE;
BEGIN
    SELECT start_date INTO v_start_date FROM schema3.emp WHERE id = p_id;
    RETURN TRUNC(SYSDATE - v_start_date);
END;
/
CREATE OR REPLACE FUNCTION schema4.get_item_price(p_item_id NUMBER) RETURN NUMBER AS -- New function
BEGIN
    RETURN 10.50; -- Mock value
END;
/
CREATE OR REPLACE FUNCTION schema4.is_high_pay(p_grade CHAR) RETURN BOOLEAN AS -- New function
BEGIN
    RETURN p_grade = 'A1';
END;
/

-- === Packages (8 - 4 Specs, 4 Bodies) ===
CREATE OR REPLACE PACKAGE schema3.emp_pkg AS
    PROCEDURE log_emp(p_id NUMBER); -- Different signature
    FUNCTION get_emp_status(p_id NUMBER) RETURN VARCHAR2; -- New element
END emp_pkg;
/
CREATE OR REPLACE PACKAGE BODY schema3.emp_pkg AS
    PROCEDURE log_emp(p_id NUMBER) IS
    BEGIN
        DBMS_OUTPUT.PUT_LINE('Emp ID: ' || p_id);
    END;
    FUNCTION get_emp_status(p_id NUMBER) RETURN VARCHAR2 IS
    BEGIN
        RETURN 'ACTIVE';
    END;
END emp_pkg;
/
CREATE OR REPLACE PACKAGE schema4.salary_pkg AS
    PROCEDURE log_salary(p_id NUMBER); -- Different signature
    PROCEDURE update_grade(p_id NUMBER, p_grade CHAR); -- New element
END salary_pkg;
/
CREATE OR REPLACE PACKAGE BODY schema4.salary_pkg AS
    PROCEDURE log_salary(p_id NUMBER) IS
    BEGIN
        DBMS_OUTPUT.PUT_LINE('Salary ID: ' || p_id);
    END;
    PROCEDURE update_grade(p_id NUMBER, p_grade CHAR) IS
    BEGIN
        NULL; -- Placeholder
    END;
END salary_pkg;
/

-- === Triggers (2) ===
CREATE OR REPLACE TRIGGER schema3.trg_auto_loc_id
    BEFORE INSERT ON schema3.locations
    FOR EACH ROW
BEGIN
    IF :NEW.loc_id IS NULL THEN
        :NEW.loc_id := schema3.client_seq.NEXTVAL;
    END IF;
END;
/

CREATE OR REPLACE TRIGGER schema4.trg_pre_txn_date
    BEFORE INSERT ON schema4.transactions
    FOR EACH ROW
BEGIN
    :NEW.txn_date := SYSTIMESTAMP;
END;
/
