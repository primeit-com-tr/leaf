CREATE OR REPLACE PROCEDURE schema3.recreate_my_table(plan_name VARCHAR2) IS
BEGIN
    -- Try drop table, ignore any error
    BEGIN
        EXECUTE IMMEDIATE 'DROP TABLE schema3.my_table PURGE';
    EXCEPTION
        WHEN OTHERS THEN
            NULL; -- ignore drop errors
    END;

    -- Create table
    EXECUTE IMMEDIATE '
        CREATE TABLE schema3.my_table (
            id        NUMBER PRIMARY KEY,
            name      VARCHAR2(100),
            created_at DATE DEFAULT SYSDATE
        )
    ';
END;
/
